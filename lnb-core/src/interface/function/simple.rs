use crate::{
    error::FunctionError,
    interface::function::{FunctionDescriptor, FunctionResponse},
};

use std::sync::Arc;

use futures::future::BoxFuture;
use serde_json::Value;

pub type ArcSimpleFunction = Arc<dyn SimpleFunction + 'static>;

pub trait SimpleFunction: Send + Sync {
    /// この `SimpleFunction` のディスクリプタを返す。
    fn get_descriptor(&self) -> FunctionDescriptor;

    /// Function を実行する。
    fn call<'a>(&'a self, id: &str, params: Value) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>>;
}
