use crate::{
    function::{DailyPrivateConfig, ExchangeRateConfig, GetIllustUrlConfig, ImageGeneratorConfig},
    shiyu::ReminderConfig,
};

use std::{collections::HashMap, path::PathBuf};

use lnb_discord_client::DiscordLnbClientConfig;
use lnb_mastodon_client::MastodonLnbClientConfig;
use serde::Deserialize;
use serde_json::Value;

/// config.yaml
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub client: AppConfigClient,
    pub tools: AppConfigTools,
    pub llm: AppConfigLlm,
    pub storage: AppConfigStorage,
    pub assistant: AppConfigAssistant,
    pub reminder: ReminderConfig,
}

/// [client]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AppConfigClient {
    pub mastodon: Option<MastodonLnbClientConfig>,
    pub discord: Option<DiscordLnbClientConfig>,
}

/// [tool]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AppConfigTools {
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
    pub default: String,
    pub models: HashMap<String, AppConfigLlmModel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfigLlmModel {
    pub backend: AppConfigLlmBackend,
    pub config: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppConfigLlmBackend {
    Openai,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppConfigLlmOpenaiApi {
    ChatCompletion,
    Resnposes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfigAssistant {
    pub system_role: String,

    #[serde(default = "Default::default")]
    pub sensitive_marker: String,
}
