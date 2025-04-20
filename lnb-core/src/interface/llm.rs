use crate::{
    error::LlmError,
    interface::function::FunctionDescriptor,
    model::{conversation::IncompleteConversation, message::MessageToolCalling},
};

use std::sync::Arc;

use futures::future::BoxFuture;
use serde::Deserialize;

pub type ArcLlm = Arc<dyn Llm + 'static>;

pub trait Llm: Send + Sync {
    /// `Conversation` を送信する。
    fn send_conversation<'a>(
        &'a self,
        conversation: &'a IncompleteConversation,
        function_descriptors: &'a [&'a FunctionDescriptor],
    ) -> BoxFuture<'a, Result<LlmUpdate, LlmError>>;
}

#[derive(Debug, Clone)]
pub enum LlmUpdate {
    Finished(LlmAssistantResponse),
    LengthCut(LlmAssistantResponse),
    ToolCalling(Vec<MessageToolCalling>),
    Filtered,
}

/// assistant role としての応答内容。
#[derive(Debug, Clone, Deserialize)]
pub struct LlmAssistantResponse {
    pub text: String,
    pub language: Option<String>,
    pub sensitive: Option<bool>,
}
