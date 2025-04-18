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

use futures::future::BoxFuture;

pub type BoxComplexFunction = Box<dyn ComplexFunction + 'static>;

pub trait ComplexFunction: Send + Sync {
    /// この `ComplexFunction` のディスクリプタを返す。
    fn get_descriptor(&self) -> FunctionDescriptor;

    /// Function を実行する。
    fn call<'a>(
        &'a self,
        context: &'a Context,
        incomplete: &'a IncompleteConversation,
        user_role: &'a UserRole,
        tool_calling: &'a MessageToolCalling,
    ) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>>;
}

impl<T: ComplexFunction + 'static> From<T> for BoxComplexFunction {
    fn from(value: T) -> BoxComplexFunction {
        Box::new(value)
    }
}
