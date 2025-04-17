use crate::{
    error::FunctionError,
    interface::function::{FunctionDescriptor, FunctionResponse},
};

use futures::future::BoxFuture;
use serde_json::Value;

pub type BoxSimpleFunction = Box<dyn SimpleFunction + 'static>;

pub trait SimpleFunction: Send + Sync {
    /// この `SimpleFunction` のディスクリプタを返す。
    fn get_descriptor(&self) -> FunctionDescriptor;

    /// Function を実行する。
    fn call<'a>(&'a self, id: &str, params: Value) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>>;
}

impl<T: SimpleFunction + 'static> From<T> for BoxSimpleFunction {
    fn from(value: T) -> BoxSimpleFunction {
        Box::new(value)
    }
}
