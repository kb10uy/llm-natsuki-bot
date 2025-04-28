pub mod assistant;
pub mod client;
pub mod llm;
pub mod reminder;
pub mod storage;
pub mod tools;

use serde::Deserialize;

/// config.yaml
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub client: client::ConfigClient,
    pub tools: tools::ConfigTools,
    pub llm: llm::ConfigLlm,
    pub storage: storage::ConfigStorage,
    pub assistant: assistant::ConfigAssistant,
    pub reminder: reminder::ConfigReminder,
}
