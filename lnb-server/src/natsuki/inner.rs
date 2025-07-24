use crate::natsuki::{function_store::FunctionStore, llm_cache::LlmCache};

use std::{iter::once, sync::Arc};

use lnb_common::{config::assistant::ConfigAssistant, debug::debug_option_parsed, time_provider::BotDateTimeProvider};
use lnb_core::{
    context::Context,
    error::{FunctionError, ServerError},
    interface::{
        MessageContext,
        interception::{BoxInterception, InterceptionStatus},
        llm::LlmUpdate,
        storage::BoxConversationStorage,
    },
    model::{
        conversation::{
            Conversation, ConversationAttachment, ConversationId, ConversationUpdate, IncompleteConversation,
        },
        message::{AssistantMessage, FunctionResponseMessage, Message, MessageToolCalling},
    },
};
use lnb_rate_limiter::{RateLimiter, Rated};
use time::{Duration, UtcDateTime};
use tracing::{debug, info, warn};

const MAX_CONVERSATION_LOOP: usize = 8;

pub struct NatsukiInner {
    storage: BoxConversationStorage,
    rate_limiter: Option<RateLimiter>,
    llm_cache: LlmCache,
    function_store: FunctionStore,
    interceptions: Vec<BoxInterception>,
    context: Context,
}

impl NatsukiInner {
    pub fn new(
        storage: BoxConversationStorage,
        rate_limiter: Option<RateLimiter>,
        llm_cache: LlmCache,
        function_store: FunctionStore,
        interceptions: Vec<BoxInterception>,
        assistant_identity: &ConfigAssistant,
    ) -> Result<NatsukiInner, ServerError> {
        let context = {
            let mut dtp = BotDateTimeProvider::new();
            let offset_days = debug_option_parsed("datetime_offset")
                .map_err(FunctionError::by_serialization)?
                .unwrap_or(0);
            if offset_days != 0 {
                warn!("day offset: {offset_days}");
                dtp.set_offset(Duration::days(offset_days as i64));
            }

            Context {
                datetime_provider: Arc::new(dtp),
                system_role: assistant_identity.system_role.clone().into(),
                sensitive_marker: assistant_identity.sensitive_marker.clone().into(),
            }
        };

        Ok(NatsukiInner {
            storage,
            rate_limiter,
            llm_cache,
            function_store,
            interceptions,
            context,
        })
    }

    pub async fn process_conversation(
        &self,
        context: MessageContext,
        conversation_id: ConversationId,
        new_messages: Vec<Message>,
    ) -> Result<ConversationUpdate, ServerError> {
        // TODO: Context に UserRole を統合する
        if !self.ensure_in_rate(&context).await {
            return Err(ServerError::RateLimitExceeded);
        }

        if !matches!(new_messages.last(), Some(Message::User(_))) {
            return Err(ServerError::MustEndsWithUserMessage);
        }

        let conversation = self
            .storage
            .fetch_content_by_id(conversation_id)
            .await?
            .ok_or_else(|| ServerError::ConversationNotFound(conversation_id))?;
        let mut incomplete_conversation = IncompleteConversation::start(conversation);
        incomplete_conversation.extend_messages(new_messages);

        // interception updates
        // 後から追加した方が前のものを "wrap" する (axum などと同じ)ので逆順
        for interception in self.interceptions.iter().rev() {
            let status = interception.before_llm(&context, &mut incomplete_conversation).await?;
            match status {
                InterceptionStatus::Continue => continue,
                InterceptionStatus::Bypass => break,
                InterceptionStatus::Complete(message) => {
                    debug!("interceptor reported conversation completion");
                    return Ok(incomplete_conversation.finish(message));
                }
                InterceptionStatus::Abort => {
                    debug!("interceptor reported conversation abortion");
                    return Err(ServerError::ConversationAborted);
                }
            }
        }

        // LLM updates
        let model = incomplete_conversation.current_model();
        let llm = self.llm_cache.get(model).await.map_err(ServerError::by_internal)?;
        debug!("using model {model:?}");

        let function_descriptors: Vec<_> = self.function_store.descriptors().collect();
        debug!("registered functions: {}", function_descriptors.len());

        let mut sent_count = 0;
        loop {
            // 超過したらエラー
            if sent_count >= MAX_CONVERSATION_LOOP {
                return Err(ServerError::TooMuchConversationCall);
            }

            let update = llm
                .send_conversation(&incomplete_conversation, &function_descriptors)
                .await?;
            sent_count += 1;

            match update {
                // 正常終了
                LlmUpdate::Finished(finished) => {
                    debug!("conversation finished");
                    let (text, is_sensitive) = self.strip_sensitive_text(finished.text, finished.sensitive);
                    return Ok(incomplete_conversation.finish(AssistantMessage {
                        text,
                        is_sensitive,
                        language: finished.language,
                        skip_llm: false,
                    }));
                }

                // 続行
                LlmUpdate::LengthCut(cut) => {
                    let (text, is_sensitive) = self.strip_sensitive_text(cut.text, cut.sensitive);
                    debug!("conversation cut, continuing: {text}");
                    incomplete_conversation.push_assistant(AssistantMessage {
                        text,
                        is_sensitive,
                        language: cut.language,
                        skip_llm: false,
                    });
                }

                // Tool Calling
                LlmUpdate::ToolCalling(tool_callings) => {
                    debug!("conversation requested tool calling");
                    let call_message = Message::new_function_calls(tool_callings.clone());
                    let (response_messages, called_attachments) = self
                        .process_tool_callings(&context, &incomplete_conversation, tool_callings)
                        .await?;

                    let extending_messages = once(call_message).chain(response_messages.into_iter().map(|m| m.into()));
                    incomplete_conversation.extend_messages(extending_messages);
                    incomplete_conversation.extend_attachments(called_attachments);
                }

                // 強制終了
                LlmUpdate::Filtered => {
                    debug!("conversation filtered");
                    return Ok(incomplete_conversation.finish(AssistantMessage {
                        text: "(filtered)".to_string(),
                        is_sensitive: true,
                        language: None,
                        skip_llm: false,
                    }));
                }
            }
        }
    }

    fn strip_sensitive_text(&self, original: String, explicit_sensitive: Option<bool>) -> (String, bool) {
        match explicit_sensitive {
            Some(v) => (original, v),
            None if self.context.sensitive_marker.is_empty() => (original, false),
            _ => match original.strip_prefix(&*self.context.sensitive_marker) {
                Some(stripped) => (stripped.to_string(), true),
                None => (original, false),
            },
        }
    }

    async fn ensure_in_rate(&self, context: &MessageContext) -> bool {
        let Some(rate_limiter) = &self.rate_limiter else {
            return true;
        };
        let Some(identity) = context.identity() else {
            return true;
        };

        let rated = rate_limiter.check(UtcDateTime::now(), identity).await;
        matches!(rated, Rated::Success)
    }

    async fn process_tool_callings(
        &self,
        context: &MessageContext,
        incomplete_conversation: &IncompleteConversation,
        tool_callings: Vec<MessageToolCalling>,
    ) -> Result<(Vec<FunctionResponseMessage>, Vec<ConversationAttachment>), ServerError> {
        let mut responses = vec![];
        let mut attachments = vec![];
        for tool_calling in tool_callings {
            let (id, name) = (tool_calling.id.clone(), tool_calling.name.clone());
            info!("calling tool {name} (id: {id})");

            let Some(response) = self
                .function_store
                .find_call(tool_calling, context, incomplete_conversation)
                .await
            else {
                warn!("tool {name} not found, skipping");
                continue;
            };
            let response = response?;

            responses.push(FunctionResponseMessage {
                id,
                name,
                result: response.result,
            });
            attachments.extend(response.attachments);
        }

        Ok((responses, attachments))
    }

    pub async fn new_conversation(&self) -> Result<ConversationId, ServerError> {
        let system_message = Message::new_system(self.context.system_role.to_string());
        let conversation = Conversation::new_now(Some(system_message));
        self.storage.upsert(&conversation, None).await?;
        Ok(conversation.id())
    }

    pub async fn restore_conversation(&self, context_key: &str) -> Result<Option<ConversationId>, ServerError> {
        let conversation_id = self.storage.fetch_id_by_context_key(context_key).await?;
        Ok(conversation_id)
    }

    pub async fn save_conversation(&self, update: ConversationUpdate, context_key: &str) -> Result<(), ServerError> {
        let current_conversation = self
            .storage
            .fetch_content_by_id(update.id())
            .await?
            .ok_or_else(|| ServerError::ConversationNotFound(update.id()))?;
        let updated_conversation = update.complete_conversation_with(current_conversation);
        self.storage.upsert(&updated_conversation, Some(context_key)).await?;
        Ok(())
    }
}
