use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigAssistant {
    pub system_role: String,

    #[serde(default = "Default::default")]
    pub sensitive_marker: String,
}
