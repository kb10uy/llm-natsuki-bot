mod api;
mod application;
mod config;
mod jwt_auth;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use lnb_common::persistence::{RedisReminderDb, SqliteConversationDb};
use tokio::{fs::read_to_string, net::TcpListener};

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

    let application = application::Application {
        conversation: SqliteConversationDb::connect(&config.storage).await?,
        reminder: RedisReminderDb::connect(&config.reminder.redis_address).await?,
    };
    let app_service = api::routes(&config.admin_api).with_state(application);

    let listener = TcpListener::bind(config.admin_api.bind_address).await?;
    axum::serve(listener, app_service).await?;
    Ok(())
}

async fn load_config(path: impl AsRef<Path>) -> Result<config::Config> {
    let config_str = read_to_string(path).await.context("failed to read config file")?;
    serde_json::from_str(&config_str).context("failed to parse config")
}
