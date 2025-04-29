pub mod admin_api;
pub mod assistant;
pub mod client;
pub mod llm;
pub mod reminder;
pub mod storage;
pub mod tools;

use std::{fs::read_to_string, io::Error as IoError, path::Path};

use serde::Deserialize;
use serde_json::Error as SerdeJsonError;
use thiserror::Error as ThisError;

/// config.yaml
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub client: client::ConfigClient,
    pub tools: tools::ConfigTools,
    pub llm: llm::ConfigLlm,
    pub storage: storage::ConfigStorage,
    pub admin_api: admin_api::ConfigAdminApi,
    pub assistant: assistant::ConfigAssistant,
    pub reminder: reminder::ConfigReminder,
}

pub fn load_config(path: impl AsRef<Path>) -> Result<Config, ConfigError> {
    let config_str = read_to_string(path).map_err(ConfigError::Io)?;
    let config = serde_json::from_str(&config_str).map_err(ConfigError::Serialization)?;
    Ok(config)
}

#[derive(Debug, ThisError)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(IoError),

    #[error("io error: {0}")]
    Serialization(SerdeJsonError),
}
