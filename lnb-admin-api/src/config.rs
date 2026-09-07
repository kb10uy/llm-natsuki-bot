use std::{fs::read_to_string, io::Error as IoError, net::SocketAddr, path::Path};

use lnb_persistence_sqlite::ConfigStorageSqlite;
use lnb_reminder_redis::ConfigReminder;
use serde::Deserialize;
use serde_json::Error as SerdeJsonError;
use thiserror::Error as ThisError;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigAdminApiService {
    pub storage: ConfigAdminStorage,
    pub admin_api: ConfigAdminApi,
    pub reminder: ConfigReminder,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigAdminStorage {
    pub sqlite: ConfigStorageSqlite,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigAdminApi {
    pub bind_address: SocketAddr,
    pub jwt_auth: Option<ConfigAdminApiJwtAuth>,
    pub cors: Option<ConfigAdminApiCors>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigAdminApiJwtAuth {
    pub jwt_header_name: String,
    pub jwks_url: Url,
    pub audience: String,
    pub allowed_subjects: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigAdminApiCors {
    pub allowed_origins: Vec<String>,
}

pub fn load_admin_api_config(path: impl AsRef<Path>) -> Result<ConfigAdminApiService, ConfigError> {
    let config_str = read_to_string(path).map_err(ConfigError::Io)?;
    serde_json::from_str(&config_str).map_err(ConfigError::Serialization)
}

#[derive(Debug, ThisError)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(IoError),

    #[error("serialization error: {0}")]
    Serialization(SerdeJsonError),
}
