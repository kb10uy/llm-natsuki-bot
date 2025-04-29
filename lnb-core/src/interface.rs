pub mod client;
pub mod function;
pub mod interception;
pub mod llm;
pub mod reminder;
pub mod server;
pub mod storage;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

// TODO: Serialize/Deserialize にしないと RPC 化に対応できない
#[derive(Debug)]
pub struct Context {
    unique_identity: Option<String>,
    values: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Context {
    pub fn new_user(unique_identity: impl Into<String>) -> Context {
        Context {
            unique_identity: Some(unique_identity.into()),
            values: HashMap::new(),
        }
    }

    pub fn new_system() -> Context {
        Context {
            unique_identity: None,
            values: HashMap::new(),
        }
    }

    pub fn identity(&self) -> Option<&str> {
        self.unique_identity.as_deref()
    }

    pub fn set<T: Any + Send + Sync>(&mut self, value: T) {
        self.values.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.values.get(&TypeId::of::<T>()).and_then(|v| v.downcast_ref())
    }
}
