pub mod client;
pub mod function;
pub mod interception;
pub mod llm;
pub mod reminder;
pub mod server;
pub mod storage;

use std::collections::HashMap;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Error as SerdeJsonError, Value};

use crate::model::conversation::UserRole;

pub trait Extension: Serialize + DeserializeOwned {
    const NAME: &'static str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    unique_identity: Option<String>,
    role: UserRole,
    values: HashMap<String, Value>,
}

impl Context {
    pub fn new_user(unique_identity: impl Into<String>, role: UserRole) -> Context {
        Context {
            unique_identity: Some(unique_identity.into()),
            role,
            values: HashMap::new(),
        }
    }

    pub fn new_system() -> Context {
        Context {
            unique_identity: None,
            role: UserRole::Privileged,
            values: HashMap::new(),
        }
    }

    pub fn identity(&self) -> Option<&str> {
        self.unique_identity.as_deref()
    }

    pub fn role(&self) -> &UserRole {
        &self.role
    }

    pub fn set<T: Extension>(&mut self, value: T) -> Result<(), SerdeJsonError> {
        let value = serde_json::to_value(value)?;
        self.values.insert(T::NAME.to_string(), value);
        Ok(())
    }

    pub fn get<T: Extension>(&self) -> Result<Option<T>, SerdeJsonError> {
        self.values
            .get(T::NAME)
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()
    }
}
