use crate::{
    error::LlmError,
    interface::function::simple::SimpleFunctionDescriptor,
    model::{conversation::IncompleteConversation, message::MessageFunctionCall},
};

use std::fmt::Debug;

use futures::future::BoxFuture;
use serde::Deserialize;

#[allow(dead_code)]
pub trait Llm: Send + Sync + Debug {
    /// `SimpleFunction` の追加を告知する。
    fn add_simple_function(&self, descriptor: SimpleFunctionDescriptor) -> BoxFuture<'_, ()>;

    /// `Conversation` を送信する。
    fn send_conversation<'a>(
        &'a self,
        conversation: &'a IncompleteConversation,
    ) -> BoxFuture<'a, Result<LlmUpdate, LlmError>>;
}

/// Conversation を送信した結果生成された内容。
#[derive(Debug, Clone)]
pub struct LlmUpdate {
    pub response: Option<LlmAssistantResponse>,
    pub tool_callings: Option<Vec<MessageFunctionCall>>,
}

/// assistant role としての応答内容。
#[derive(Debug, Clone, Deserialize)]
pub struct LlmAssistantResponse {
    pub text: String,
    pub language: Option<String>,
    pub sensitive: Option<bool>,
}
