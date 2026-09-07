use lnb_persistence_sqlite::ConfigStorageSqlite;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigStorage {
    pub backend: ConfigStorageBackend,
    pub sqlite: ConfigStorageSqlite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigStorageBackend {
    Sqlite,
    Memory,
}
