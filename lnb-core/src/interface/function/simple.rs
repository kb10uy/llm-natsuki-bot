use crate::{
    error::FunctionError,
    model::{conversation::ConversationAttachment, schema::DescribedSchema},
};

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type BoxSimpleFunction = Box<dyn SimpleFunction + 'static>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimpleFunctionDescriptor {
    pub name: String,
    pub description: String,
    pub parameters: DescribedSchema,
}

#[derive(Debug, Clone, Default)]
pub struct SimpleFunctionResponse {
    pub result: Value,
    pub attachments: Vec<ConversationAttachment>,
}

pub trait SimpleFunction: Send + Sync {
    /// この `SimpleFunction` のディスクリプタを返す。
    fn get_descriptor(&self) -> SimpleFunctionDescriptor;

    /// Function を実行する。
    fn call<'a>(&'a self, id: &str, params: Value) -> BoxFuture<'a, Result<SimpleFunctionResponse, FunctionError>>;
}

impl<T: SimpleFunction + 'static> From<T> for BoxSimpleFunction {
    fn from(value: T) -> BoxSimpleFunction {
        Box::new(value)
    }
}
