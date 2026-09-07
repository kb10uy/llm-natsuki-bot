use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigClientDiscord {
    pub token: String,
    pub max_length: usize,
}
