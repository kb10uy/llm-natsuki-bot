mod api;
mod application;
mod jwt_auth;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use lnb_common::{
    config::load_config,
    persistence::{RedisReminderDb, SqliteConversationDb},
};
use tokio::net::TcpListener;

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
    let config = load_config(&args.config)?;

    let application = application::Application {
        conversation: SqliteConversationDb::connect(&config.storage.sqlite).await?,
        reminder: RedisReminderDb::connect(&config.reminder).await?,
    };
    let app_service = api::routes(&config.admin_api).with_state(application);

    let listener = TcpListener::bind(config.admin_api.bind_address).await?;
    axum::serve(listener, app_service).await?;
    Ok(())
}
