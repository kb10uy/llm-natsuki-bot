use serde::Deserialize;

/// [client]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ConfigClient {
    pub mastodon: Option<ConfigClientMastodon>,
    pub discord: Option<ConfigClientDiscord>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigClientMastodon {
    pub server_url: String,
    pub token: String,
    pub sensitive_spoiler: String,
    pub max_length: usize,
    pub remote_fetch_delay_seconds: usize,
    pub math_renderer: ConfigClientMathRenderer,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigClientDiscord {
    pub token: String,
    pub max_length: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigClientMathRenderer {
    pub endpoint: String,
    pub scale: f64,
}
