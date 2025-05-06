use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum UserRole {
    Privileged,
    Normal,

    #[serde(untagged)]
    Scoped(BTreeSet<String>),
}

impl UserRole {
    pub fn scoped_with(scopes: impl IntoIterator<Item = impl Into<String>>) -> UserRole {
        UserRole::Scoped(scopes.into_iter().map(|s| s.into()).collect())
    }

    pub fn accepts(&self, scope: &str) -> bool {
        match self {
            UserRole::Privileged => true,
            UserRole::Scoped(scopes) => scopes.contains(scope),
            UserRole::Normal => false,
        }
    }
}
