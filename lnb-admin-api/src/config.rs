use std::{net::SocketAddr, path::PathBuf};

use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub admin_api: ConfigAdminApi,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigAdminApi {
    pub bind_address: SocketAddr,
    pub jwt_auth: Option<ConfigAdminApiJwtAuth>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigAdminApiJwtAuth {
    pub jwt_header_name: String,
    pub jwks_url: Url,
    pub audience: String,
    pub allowed_subjects: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfigStorage {
    pub backend: AppConfigStorageBackend,
    pub sqlite: AppConfigStorageSqlite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppConfigStorageBackend {
    Sqlite,
    Memory,
}

/// [storage.sqlite]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AppConfigStorageSqlite {
    pub filepath: PathBuf,
}
