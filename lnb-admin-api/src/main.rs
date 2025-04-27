mod api;
mod config;
mod jwt_auth;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use tokio::{fs::read_to_string, net::TcpListener};
use tracing::info;

#[derive(Debug, Clone, Parser)]
#[clap(author, version)]
struct Arguments {
    #[clap(short, long, default_value = "./config.json")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Arguments::parse();
    let config = load_config(&args.config).await?;

    let app = {
        let mut router = api::routes();

        // JWT Auth
        if let Some(auth_config) = config.admin_api.jwt_auth {
            router = router.layer(jwt_auth::JwtAuthLayer::new(auth_config));
            info!("JWT authentication enabled");
        }

        router
    };

    let listener = TcpListener::bind(config.admin_api.bind_address).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn load_config(path: impl AsRef<Path>) -> Result<config::Config> {
    let config_str = read_to_string(path).await.context("failed to read config file")?;
    serde_json::from_str(&config_str).context("failed to parse config")
}
