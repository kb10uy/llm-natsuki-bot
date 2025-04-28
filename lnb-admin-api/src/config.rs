use std::net::SocketAddr;

use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub admin_api: ConfigAdminApi,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigAdminApi {
    pub bind_address: SocketAddr,
    pub jwt_auth: Option<ConfigAdminApiJwtAuth>,
    pub cors: Option<ConfigAdminApiCors>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigAdminApiJwtAuth {
    pub jwt_header_name: String,
    pub jwks_url: Url,
    pub audience: String,
    pub allowed_subjects: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigAdminApiCors {
    pub allowed_origins: Vec<String>,
}
