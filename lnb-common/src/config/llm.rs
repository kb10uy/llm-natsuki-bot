use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigLlm {
    pub default: String,
    pub models: HashMap<String, ConfigLlmModel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigLlmModel {
    pub backend: ConfigLlmBackend,
    pub config: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigLlmBackend {
    Openai,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigLlmOpenaiApi {
    ChatCompletion,
    Resnposes,
}
