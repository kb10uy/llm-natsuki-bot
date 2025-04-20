mod chat_completion;
mod responses;

pub use chat_completion::ChatCompletionBackend;
pub use responses::ResponsesBackend;
use serde_json::Value;

use crate::llm::{ASSISTANT_RESPONSE_SCHEMA, convert_json_schema};

use std::sync::{Arc, LazyLock};

use async_openai::{Client, config::OpenAIConfig, types::ResponseFormatJsonSchema};
use lnb_core::{APP_USER_AGENT, error::LlmError, interface::llm::ArcLlm};
use serde::Deserialize;

static RESPONSE_JSON_SCHEMA: LazyLock<ResponseFormatJsonSchema> = LazyLock::new(|| ResponseFormatJsonSchema {
    name: "response".into(),
    description: Some("response from assistant".into()),
    schema: Some(convert_json_schema(&ASSISTANT_RESPONSE_SCHEMA)),
    strict: Some(true),
});

#[derive(Debug, Clone, Deserialize)]
pub struct OpenaiModelConfig {
    pub api: OpenaiModelConfigApi,
    pub endpoint: String,
    pub token: String,
    pub model: String,
    pub enable_tool: bool,
    pub structured: bool,
    pub max_token: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenaiModelConfigApi {
    ChatCompletion,
    Responses,
}

pub async fn create_openai_llm(config_value: Value) -> Result<ArcLlm, LlmError> {
    let config: OpenaiModelConfig = serde_json::from_value(config_value).map_err(LlmError::by_format)?;
    match config.api {
        OpenaiModelConfigApi::ChatCompletion => Ok(Arc::new(ChatCompletionBackend::new(config).await?)),
        OpenaiModelConfigApi::Responses => Ok(Arc::new(ResponsesBackend::new(config).await?)),
    }
}

async fn create_openai_client(token: &str, endpoint: &str) -> Result<Client<OpenAIConfig>, LlmError> {
    let config = OpenAIConfig::new().with_api_key(token).with_api_base(endpoint);
    let http_client = reqwest::ClientBuilder::new()
        .user_agent(APP_USER_AGENT)
        .build()
        .map_err(LlmError::by_communication)?;

    let client = Client::with_config(config).with_http_client(http_client);
    Ok(client)
}
