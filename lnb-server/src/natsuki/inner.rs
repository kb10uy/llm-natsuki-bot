use crate::{
    config::AppConfigAssistantIdentity,
    natsuki::{function_store::FunctionStore, llm_cache::LlmCache},
};

use std::iter::once;

use lnb_core::{
    error::ServerError,
    interface::{
        Context,
        interception::{BoxInterception, InterceptionStatus},
        llm::LlmUpdate,
        storage::BoxConversationStorage,
    },
    model::{
        conversation::{
            Conversation, ConversationAttachment, ConversationId, ConversationUpdate, IncompleteConversation, UserRole,
        },
        message::{AssistantMessage, FunctionResponseMessage, Message, MessageToolCalling},
    },
};
use tracing::{debug, info, warn};

const MAX_CONVERSATION_LOOP: usize = 8;

pub struct NatsukiInner {
    storage: BoxConversationStorage,
    llm_cache: LlmCache,
    function_store: FunctionStore,
    interceptions: Vec<BoxInterception>,
    system_role: String,
    sensitive_marker: String,
}

impl NatsukiInner {
    pub fn new(
        storage: BoxConversationStorage,
        llm_cache: LlmCache,
        function_store: FunctionStore,
        interceptions: Vec<BoxInterception>,
        assistant_identity: &AppConfigAssistantIdentity,
    ) -> Result<NatsukiInner, ServerError> {
        Ok(NatsukiInner {
            storage,
            llm_cache,
            function_store,
            interceptions,
            system_role: assistant_identity.system_role.clone(),
            sensitive_marker: assistant_identity.sensitive_marker.clone(),
        })
    }

    pub async fn process_conversation(
        &self,
        context: Context,
        conversation_id: ConversationId,
        new_messages: Vec<Message>,
        user_role: UserRole,
    ) -> Result<ConversationUpdate, ServerError> {
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
            let status = interception
                .before_llm(&context, &mut incomplete_conversation, &user_role)
                .await?;
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
        let llm = self
            .llm_cache
            .get(incomplete_conversation.current_model())
            .await
            .map_err(ServerError::by_internal)?;
        let function_descriptors: Vec<_> = self.function_store.descriptors().collect();
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
                        .process_tool_callings(&context, &incomplete_conversation, &user_role, tool_callings)
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
            None if self.sensitive_marker.is_empty() => (original, false),
            _ => match original.strip_prefix(&self.sensitive_marker) {
                Some(stripped) => (stripped.to_string(), true),
                None => (original, false),
            },
        }
    }

    async fn process_tool_callings(
        &self,
        context: &Context,
        incomplete_conversation: &IncompleteConversation,
        user_role: &UserRole,
        tool_callings: Vec<MessageToolCalling>,
    ) -> Result<(Vec<FunctionResponseMessage>, Vec<ConversationAttachment>), ServerError> {
        let mut responses = vec![];
        let mut attachments = vec![];
        for tool_calling in tool_callings {
            let (id, name) = (tool_calling.id.clone(), tool_calling.name.clone());
            info!("calling tool {name} (id: {id})");

            let Some(response) = self
                .function_store
                .find_call(tool_calling, context, incomplete_conversation, user_role)
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
        let system_message = Message::new_system(self.system_role.clone());
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
