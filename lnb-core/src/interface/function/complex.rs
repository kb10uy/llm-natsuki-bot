use crate::{
    error::FunctionError,
    interface::{
        Context,
        function::{FunctionDescriptor, FunctionResponse},
    },
    model::{
        conversation::{IncompleteConversation, UserRole},
        message::MessageToolCalling,
    },
};

use std::sync::Arc;

use futures::future::BoxFuture;

pub type ArcComplexFunction = Arc<dyn ComplexFunction + 'static>;

pub trait ComplexFunction: Send + Sync {
    /// この `ComplexFunction` のディスクリプタを返す。
    fn get_descriptor(&self) -> FunctionDescriptor;

    /// Function を実行する。
    fn call<'a>(
        &'a self,
        context: &'a Context,
        incomplete: &'a IncompleteConversation,
        user_role: &'a UserRole,
        tool_calling: MessageToolCalling,
    ) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>>;
}
