use crate::{
    error::FunctionError,
    interface::function::{FunctionDescriptor, FunctionResponse},
};

use futures::future::BoxFuture;
use serde_json::Value;

pub type BoxComplexFunction = Box<dyn ComplexFunction + 'static>;

pub trait ComplexFunction: Send + Sync {
    /// この `ComplexFunction` のディスクリプタを返す。
    fn get_descriptor(&self) -> FunctionDescriptor;

    /// Function を実行する。
    fn call<'a>(&'a self, id: &str, params: Value) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>>;
}

impl<T: ComplexFunction + 'static> From<T> for BoxComplexFunction {
    fn from(value: T) -> BoxComplexFunction {
        Box::new(value)
    }
}
