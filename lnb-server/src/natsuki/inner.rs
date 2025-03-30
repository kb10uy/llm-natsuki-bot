use crate::config::AppConfigAssistantIdentity;

use std::collections::HashMap;

use lnb_core::{
    error::ServerError,
    interface::{function::simple::SimpleFunction, llm::Llm, storage::ConversationStorage},
    model::{
        conversation::{Conversation, ConversationAttachment, ConversationUpdate, IncompleteConversation},
        message::{AssistantMessage, FunctionResponseMessage, Message, MessageFunctionCall, UserMessage},
    },
};
use tokio::sync::Mutex;
use tracing::{info, warn};

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
        conversation: Conversation,
        user_message: UserMessage,
    ) -> Result<ConversationUpdate, ServerError> {
        let mut incomplete_conversation = IncompleteConversation::start(conversation, user_message);

        let first_update = self.llm.send_conversation(&incomplete_conversation).await?;
        let (assistant_update, attachments) = if let Some(tool_callings) = first_update.tool_callings {
            let call_message = Message::new_function_calls(tool_callings.clone());
            let (response_messages, attachments) = self.process_tool_callings(tool_callings).await?;

            incomplete_conversation.latest_messages.push(call_message);
            incomplete_conversation
                .latest_messages
                .extend(response_messages.into_iter().map(|m| m.into()));

            let second_update = self.llm.send_conversation(&incomplete_conversation).await?;
            (second_update, attachments)
        } else {
            (first_update, vec![])
        };

        let Some(response) = assistant_update.response else {
            return Err(ServerError::ChatResponseExpected);
        };

        let (text, is_sensitive) = match response.sensitive {
            Some(v) => (response.text, v),
            None if self.sensitive_marker.is_empty() => (response.text, false),
            _ => match response.text.strip_prefix(&self.sensitive_marker) {
                Some(stripped) => (stripped.to_string(), true),
                None => (response.text, false),
            },
        };

        Ok(incomplete_conversation.finish(
            AssistantMessage {
                text,
                is_sensitive,
                language: response.language,
            },
            attachments,
        ))
    }

    async fn process_tool_callings(
        &self,
        tool_callings: Vec<MessageFunctionCall>,
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

    pub fn new_conversation(&self) -> Result<ConversationId, ServerError> {
        let system_message = Message::new_system(self.system_role.clone());
        Conversation::new_now(Some(system_message))
    }

    pub async fn restore_conversation(
        &self,
        platform: &str,
        context: &str,
    ) -> Result<Option<Conversation>, ServerError> {
        let conversation = self.storage.find_by_platform_context(platform, context).await?;
        Ok(conversation)
    }

    pub async fn save_conversation(
        &self,
        conversation: &Conversation,
        platform: &str,
        context: &str,
    ) -> Result<(), ServerError> {
        self.storage.upsert(conversation, platform, context).await?;
        Ok(())
    }
}
