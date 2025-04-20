use crate::llm::openai::OpenaiModelConfig;

use std::sync::Arc;

use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::LlmError,
    interface::{
        function::FunctionDescriptor,
        llm::{Llm, LlmUpdate},
    },
    model::conversation::IncompleteConversation,
};

/// OpenAI Responses API を利用したバックエンド。
#[derive(Debug, Clone)]
pub struct ResponsesBackend(Arc<ResponsesBackendInner>);

impl ResponsesBackend {
    pub async fn new(_config: OpenaiModelConfig) -> Result<ResponsesBackend, LlmError> {
        Ok(ResponsesBackend(Arc::new(ResponsesBackendInner {})))
    }
}

impl Llm for ResponsesBackend {
    fn send_conversation<'a>(
        &'a self,
        conversation: &'a IncompleteConversation,
        function_descriptors: &'a [&'a FunctionDescriptor],
    ) -> BoxFuture<'a, Result<LlmUpdate, LlmError>> {
        let cloned = self.0.clone();
        async move { cloned.send_conversation(conversation, function_descriptors).await }.boxed()
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct ResponsesBackendInner {}

impl ResponsesBackendInner {
    async fn send_conversation(
        &self,
        _conversation: &IncompleteConversation,
        _function_descriptors: &[&FunctionDescriptor],
    ) -> Result<LlmUpdate, LlmError> {
        todo!();
    }
}
