use crate::{
    error::LlmError,
    interface::function::simple::SimpleFunctionDescriptor,
    model::{conversation::IncompleteConversation, message::MessageToolCalling},
};

use futures::future::BoxFuture;
use serde::Deserialize;

pub type BoxLlm = Box<dyn Llm + 'static>;

pub trait Llm: Send + Sync {
    /// `SimpleFunction` の追加を告知する。
    fn add_simple_function(&self, descriptor: SimpleFunctionDescriptor) -> BoxFuture<'_, ()>;

    /// `Conversation` を送信する。
    fn send_conversation<'a>(
        &'a self,
        conversation: &'a IncompleteConversation,
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

impl<T: Llm + 'static> From<T> for BoxLlm {
    fn from(value: T) -> BoxLlm {
        Box::new(value)
    }
}
