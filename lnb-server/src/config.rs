pub mod assistant;
pub mod llm;
pub mod storage;
pub mod tools;

use std::{fs::read_to_string, io::Error as IoError, path::Path};

use lnb_discord_client::ConfigClientDiscord;
use lnb_mastodon_client::ConfigClientMastodon;
pub use lnb_reminder_redis::ConfigReminder;
use serde::Deserialize;
use serde_json::Error as SerdeJsonError;
use thiserror::Error as ThisError;

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigBot {
    pub client: ConfigClient,
    pub tools: tools::ConfigTools,
    pub llm: llm::ConfigLlm,
    pub storage: storage::ConfigStorage,
    pub assistant: assistant::ConfigAssistant,
    pub reminder: ConfigReminder,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ConfigClient {
    pub mastodon: Option<ConfigClientMastodon>,
    pub discord: Option<ConfigClientDiscord>,
}

pub fn load_bot_config(path: impl AsRef<Path>) -> Result<ConfigBot, ConfigError> {
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
