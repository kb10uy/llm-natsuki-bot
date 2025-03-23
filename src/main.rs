mod application;
mod chat;
mod model;
mod platform;

use crate::{
    application::{cli::Arguments, config::load_config},
    chat::{ChatInterface, chat_completion::ChatCompletionBackend},
    platform::{ConversationPlatform, cli::CliPlatform, mastodon::MastodonPlatform},
};

use anyhow::Result;
use clap::Parser;
use futures::future::join_all;
use tokio::spawn;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Arguments::parse();
    let config = load_config(args.config).await?;

    let chat_interface = ChatInterface::<ChatCompletionBackend>::new(&config).await?;

    let mut platform_tasks = vec![];

    // CLI
    if config.platform.cli.enabled {
        let cli_platform = CliPlatform::new(&chat_interface);
        let cli_task = spawn(cli_platform.execute());
        platform_tasks.push(cli_task);
    }

    // Mastodon
    if config.platform.mastodon.enabled {
        let mastodon_platform = MastodonPlatform::new(&config.platform.mastodon, &chat_interface)?;
        let mastodon_task = spawn(mastodon_platform.execute());
        platform_tasks.push(mastodon_task);
    }

    join_all(platform_tasks).await;
    Ok(())
}
