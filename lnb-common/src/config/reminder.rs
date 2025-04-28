use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigReminder {
    pub redis_address: String,
    pub max_seconds: i64,
    pub notification_virtual_text: String,
}
