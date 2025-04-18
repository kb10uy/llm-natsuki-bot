use crate::config::AppConfigAssistantIdentity;

use std::{collections::HashMap, iter::once};

use lnb_core::{
    error::ServerError,
    interface::{
        Context,
        function::{complex::BoxComplexFunction, simple::BoxSimpleFunction},
        interception::{BoxInterception, InterceptionStatus},
        llm::{BoxLlm, LlmUpdate},
        storage::BoxConversationStorage,
    },
    model::{
        conversation::{
            Conversation, ConversationAttachment, ConversationId, ConversationUpdate, IncompleteConversation, UserRole,
        },
        message::{AssistantMessage, FunctionResponseMessage, Message, MessageToolCalling},
    },
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

const MAX_CONVERSATION_LOOP: usize = 8;

pub struct NatsukiInner {
    llm: BoxLlm,
    storage: BoxConversationStorage,
    simple_functions: RwLock<HashMap<String, BoxSimpleFunction>>,
    complex_functions: RwLock<HashMap<String, BoxComplexFunction>>,
    interceptions: RwLock<Vec<BoxInterception>>,
    system_role: String,
    sensitive_marker: String,
}

impl NatsukiInner {
    pub fn new(
        assistant_identity: &AppConfigAssistantIdentity,
        llm: BoxLlm,
        storage: BoxConversationStorage,
    ) -> Result<NatsukiInner, ServerError> {
        Ok(NatsukiInner {
            llm,
            storage,
            simple_functions: RwLock::new(HashMap::new()),
            complex_functions: RwLock::new(HashMap::new()),
            interceptions: RwLock::new(Vec::new()),
            system_role: assistant_identity.system_role.clone(),
            sensitive_marker: assistant_identity.sensitive_marker.clone(),
        })
    }

    /// `SimpleFunction` を登録する。
    pub async fn add_simple_function(&self, simple_function: BoxSimpleFunction) {
        let descriptor = simple_function.get_descriptor();

        let mut locked = self.simple_functions.write().await;
        locked.insert(descriptor.name.clone(), simple_function);
        self.llm.add_simple_function(descriptor).await;
    }

    /// `SimpleFunction` を登録する。
    pub async fn add_complex_function(&self, complex_function: BoxComplexFunction) {
        let descriptor = complex_function.get_descriptor();

        let mut locked = self.complex_functions.write().await;
        locked.insert(descriptor.name.clone(), complex_function);
        self.llm.add_simple_function(descriptor).await;
    }

    pub async fn apply_interception(&self, interception: BoxInterception) {
        let mut locked = self.interceptions.write().await;
        locked.push(interception);
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
        let interceptions = self.interceptions.read().await;
        for interception in interceptions.iter().rev() {
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
        let mut sent_count = 0;
        loop {
            // 超過したらエラー
            if sent_count >= MAX_CONVERSATION_LOOP {
                return Err(ServerError::TooMuchConversationCall);
            }

            let update = self.llm.send_conversation(&incomplete_conversation).await?;
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
        let locked_simple = self.simple_functions.read().await;
        let locked_complex = self.complex_functions.read().await;

        let mut responses = vec![];
        let mut attachments = vec![];
        for tool_calling in tool_callings {
            info!("calling tool {} (id: {})", tool_calling.name, tool_calling.id);

            let result_attachments = if let Some(simple_function) = locked_simple.get(&tool_calling.name) {
                let result = simple_function.call(&tool_calling.id, tool_calling.arguments).await?;
                responses.push(FunctionResponseMessage {
                    id: tool_calling.id,
                    name: tool_calling.name,
                    result: result.result,
                });
                result.attachments
            } else if let Some(complex_function) = locked_complex.get(&tool_calling.name) {
                let result = complex_function
                    .call(context, incomplete_conversation, user_role, &tool_calling)
                    .await?;
                responses.push(FunctionResponseMessage {
                    id: tool_calling.id,
                    name: tool_calling.name,
                    result: result.result,
                });
                result.attachments
            } else {
                warn!("tool {} not found, skipping", tool_calling.name);
                continue;
            };
            attachments.extend(result_attachments);
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
