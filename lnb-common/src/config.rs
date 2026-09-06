pub mod admin_api;
pub mod assistant;
pub mod client;
pub mod llm;
pub mod reminder;
pub mod storage;
pub mod tools;

use std::{fs::read_to_string, io::Error as IoError, path::Path};

use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Error as SerdeJsonError;
use thiserror::Error as ThisError;

/// config.yaml
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigBot {
    pub client: client::ConfigClient,
    pub tools: tools::ConfigTools,
    pub llm: llm::ConfigLlm,
    pub storage: storage::ConfigStorage,
    pub assistant: assistant::ConfigAssistant,
    pub reminder: reminder::ConfigReminder,
}

/// Backwards-compatible name for the bot process configuration.
pub type Config = ConfigBot;

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigAdminApiService {
    pub storage: storage::ConfigStorage,
    pub admin_api: admin_api::ConfigAdminApi,
    pub reminder: reminder::ConfigReminder,
}

pub fn load_bot_config(path: impl AsRef<Path>) -> Result<ConfigBot, ConfigError> {
    load_typed_config(path)
}

/// Loads the bot process configuration.
pub fn load_config(path: impl AsRef<Path>) -> Result<Config, ConfigError> {
    load_bot_config(path)
}

pub fn load_admin_api_config(path: impl AsRef<Path>) -> Result<ConfigAdminApiService, ConfigError> {
    load_typed_config(path)
}

fn load_typed_config<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, ConfigError> {
    let config_str = read_to_string(path).map_err(ConfigError::Io)?;
    let config = serde_json::from_str(&config_str).map_err(ConfigError::Serialization)?;
    Ok(config)
}

#[derive(Debug, ThisError)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(IoError),

    #[error("serialization error: {0}")]
    Serialization(SerdeJsonError),
}
