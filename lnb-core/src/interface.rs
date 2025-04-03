pub mod client;
pub mod function;
pub mod interception;
pub mod llm;
pub mod server;
pub mod storage;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

// TODO: Serialize/Deserialize にしないと RPC 化に対応できない
#[derive(Debug)]
pub struct Context {
    values: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Context {
    pub fn new() -> Context {
        Context { values: HashMap::new() }
    }

    pub fn set<T: Any + Send + Sync>(&mut self, value: T) {
        self.values.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.values.get(&TypeId::of::<T>()).and_then(|v| v.downcast_ref())
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
