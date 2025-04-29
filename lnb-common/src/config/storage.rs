use std::path::PathBuf;

use serde::Deserialize;

/// [storage]
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigStorage {
    pub backend: ConfigStorageBackend,
    pub sqlite: ConfigStorageSqlite,
}

/// [storage].backend の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigStorageBackend {
    Sqlite,
    Memory,
}

/// [storage.sqlite]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigStorageSqlite {
    pub filepath: PathBuf,
}
