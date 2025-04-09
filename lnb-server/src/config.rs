use crate::function::{DailyPrivateConfig, ExchangeRateConfig, GetIllustUrlConfig, ImageGeneratorConfig};

use std::{collections::HashMap, path::PathBuf};

use lnb_discord_client::DiscordLnbClientConfig;
use lnb_mastodon_client::MastodonLnbClientConfig;
use serde::Deserialize;

/// config.toml
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "Default::default")]
    pub client: AppConfigClient,

    #[serde(default = "Default::default")]
    pub tool: AppConfigTool,

    pub llm: AppConfigLlm,
    pub storage: AppConfigStorage,
    pub assistant: AppConfigAssistant,
}

/// [client]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AppConfigClient {
    pub mastodon: Option<MastodonLnbClientConfig>,
    pub discord: Option<DiscordLnbClientConfig>,
}

/// [tool]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AppConfigTool {
    pub image_generator: Option<ImageGeneratorConfig>,
    pub get_illust_url: Option<GetIllustUrlConfig>,
    pub exchange_rate: Option<ExchangeRateConfig>,
    pub daily_private: Option<DailyPrivateConfig>,
}

/// [storage]
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfigStorage {
    pub backend: AppConfigStorageBackend,
    pub sqlite: AppConfigStorageSqlite,
}

/// [storage].backend の種類。
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

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfigLlm {
    pub backend: AppConfigLlmBackend,
    pub openai: AppConfigLlmOpenai,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppConfigLlmBackend {
    Openai,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfigLlmOpenai {
    pub api: AppConfigLlmOpenaiApi,
    pub endpoint: String,
    pub token: String,
    pub model: String,
    pub max_token: usize,
    pub use_structured_output: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppConfigLlmOpenaiApi {
    ChatCompletion,
    Resnposes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfigAssistant {
    pub identity: String,
    pub identities: HashMap<String, AppConfigAssistantIdentity>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfigAssistantIdentity {
    pub system_role: String,

    #[serde(default = "Default::default")]
    pub sensitive_marker: String,
}
