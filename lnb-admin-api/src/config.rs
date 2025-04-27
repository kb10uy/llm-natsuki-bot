use std::{net::SocketAddr, path::PathBuf};

use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub admin_api: ConfigAdminApi,
    pub storage: ConfigStorage,
    pub reminder: ConfigReminder,
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
pub struct ConfigStorage {
    pub sqlite: ConfigStorageSqlite,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigStorageSqlite {
    pub filepath: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigReminder {
    pub redis_address: String,
}
