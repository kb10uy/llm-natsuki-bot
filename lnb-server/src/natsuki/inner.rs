use crate::config::AppConfigAssistantIdentity;

use std::{collections::HashMap, iter::once};

use lnb_core::{
    error::ServerError,
    interface::{
        function::simple::SimpleFunction,
        llm::{Llm, LlmUpdate},
        storage::ConversationStorage,
    },
    model::{
        conversation::{
            Conversation, ConversationAttachment, ConversationId, ConversationUpdate, IncompleteConversation,
        },
        message::{AssistantMessage, FunctionResponseMessage, Message, MessageToolCalling, UserMessage},
    },
};
use tokio::sync::Mutex;
use tracing::{info, warn};

const MAX_CONVERSATION_LOOP: usize = 8;

#[derive(Debug)]
pub struct NatsukiInner {
    llm: Box<dyn Llm + 'static>,
    storage: Box<dyn ConversationStorage + 'static>,
    simple_functions: Mutex<HashMap<String, Box<dyn SimpleFunction + 'static>>>,
    system_role: String,
    sensitive_marker: String,
}

impl NatsukiInner {
    pub fn new(
        assistant_identity: &AppConfigAssistantIdentity,
        llm: Box<dyn Llm + 'static>,
        storage: Box<dyn ConversationStorage + 'static>,
    ) -> Result<NatsukiInner, ServerError> {
        Ok(NatsukiInner {
            llm,
            storage,
            simple_functions: Mutex::new(HashMap::new()),
            system_role: assistant_identity.system_role.clone(),
            sensitive_marker: assistant_identity.sensitive_marker.clone(),
        })
    }

    /// `SimpleFunction` を登録する。
    pub async fn add_simple_function(&self, simple_function: impl SimpleFunction + 'static) {
        let descriptor = simple_function.get_descriptor();

        let mut locked = self.simple_functions.lock().await;
        locked.insert(descriptor.name.clone(), Box::new(simple_function));
        self.llm.add_simple_function(descriptor).await;
    }

    pub async fn process_conversation(
        &self,
        conversation_id: ConversationId,
        user_message: UserMessage,
    ) -> Result<ConversationUpdate, ServerError> {
        let conversation = self
            .storage
            .fetch_content_by_id(conversation_id)
            .await?
            .ok_or_else(|| ServerError::ConversationNotFound(conversation_id))?;
        let mut incomplete_conversation = IncompleteConversation::start(conversation, user_message);

        let mut attachments = vec![];
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
                    let (text, is_sensitive) = self.strip_sensitive_text(finished.text, finished.sensitive);
                    return Ok(incomplete_conversation.finish(
                        AssistantMessage {
                            text,
                            is_sensitive,
                            language: finished.language,
                        },
                        attachments,
                    ));
                }

                // 続行
                LlmUpdate::LengthCut(cut) => {
                    let (text, is_sensitive) = self.strip_sensitive_text(cut.text, cut.sensitive);
                    incomplete_conversation.push_assistant(AssistantMessage {
                        text,
                        is_sensitive,
                        language: cut.language,
                    });
                }

                // Tool Calling
                LlmUpdate::ToolCalling(tool_callings) => {
                    let call_message = Message::new_function_calls(tool_callings.clone());
                    let (response_messages, called_attachments) = self.process_tool_callings(tool_callings).await?;

                    let extending_messages = once(call_message).chain(response_messages.into_iter().map(|m| m.into()));
                    incomplete_conversation.extend_message(extending_messages);
                    attachments.extend(called_attachments);
                }

                // 強制終了
                LlmUpdate::Filtered => {
                    return Ok(incomplete_conversation.finish(
                        AssistantMessage {
                            text: "(filtered)".to_string(),
                            is_sensitive: true,
                            language: None,
                        },
                        attachments,
                    ));
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
        tool_callings: Vec<MessageToolCalling>,
    ) -> Result<(Vec<FunctionResponseMessage>, Vec<ConversationAttachment>), ServerError> {
        let locked = self.simple_functions.lock().await;

        let mut responses = vec![];
        let mut attachments = vec![];
        for tool_calling in tool_callings {
            info!("calling tool {} (id: {})", tool_calling.name, tool_calling.id);
            // MCP と複合するのをあとで考える
            let Some(simple_function) = locked.get(&tool_calling.name) else {
                warn!("tool {} not found, skipping", tool_calling.name);
                continue;
            };
            let result = simple_function.call(&tool_calling.id, tool_calling.arguments).await?;
            responses.push(FunctionResponseMessage {
                id: tool_calling.id,
                name: tool_calling.name,
                result: result.result,
            });
            attachments.extend(result.attachments);
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
        let completing_messages = update.into_completing_messages();

        let updated_conversation = current_conversation.push_messages(completing_messages);
        self.storage.upsert(&updated_conversation, Some(context_key)).await?;
        Ok(())
    }
}
