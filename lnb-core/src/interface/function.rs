use crate::{
    error::FunctionError,
    interface::Context,
    model::{
        conversation::{ConversationAttachment, IncompleteConversation},
        message::MessageToolCalling,
        schema::DescribedSchema,
    },
};

use std::{fmt::Debug, sync::Arc};

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type ArcFunction = Arc<dyn Function + 'static>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionDescriptor {
    pub name: String,
    pub description: String,
    pub parameters: DescribedSchema,
}

#[derive(Debug, Clone, Default)]
pub struct FunctionResponse {
    pub result: Value,
    pub attachments: Vec<ConversationAttachment>,
}

pub trait Function: Send + Sync {
    /// この `ComplexFunction` のディスクリプタを返す。
    fn get_descriptor(&self) -> FunctionDescriptor;

    /// Function を実行する。
    fn call<'a>(
        &'a self,
        context: &'a Context,
        incomplete: &'a IncompleteConversation,
        tool_calling: MessageToolCalling,
    ) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>>;
}
