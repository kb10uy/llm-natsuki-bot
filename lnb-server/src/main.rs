mod cli;
mod config;
mod function;
mod llm;
mod natsuki;
mod storage;

use crate::{
    config::AppConfig,
    function::{ExchangeRate, GetIllustUrl, ImageGenerator, LocalInfo, SelfInfo},
    llm::create_llm,
    natsuki::Natsuki,
    storage::create_storage,
};

use std::path::Path;

use anyhow::{Context as _, Result, bail};
use clap::Parser;
use config::AppConfigTool;
use futures::future::join_all;
use lnb_core::interface::client::LnbClient;
use lnb_discord_client::DiscordLnbClient;
use lnb_mastodon_client::MastodonLnbClient;
use tokio::{fs::read_to_string, spawn};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = cli::Arguments::parse();
    let config = load_config(args.config).await?;

    let natsuki = initialize_natsuki(&config).await?;
    register_simple_functions(&config.tool, &natsuki).await?;

    let mut client_tasks = vec![];

    // Mastodon
    if let Some(mastodon_config) = &config.client.mastodon {
        info!("starting Mastodon client");
        let mastodon_client = MastodonLnbClient::new(mastodon_config, natsuki.clone()).await?;
        let mastodon_task = spawn(mastodon_client.execute());
        client_tasks.push(Box::new(mastodon_task));
    }

    // Discord
    if let Some(dicsord_config) = &config.client.discord {
        info!("starting Discord client");
        let discord_client = DiscordLnbClient::new(dicsord_config, natsuki.clone()).await?;
        let discord_task = spawn(discord_client.execute());
        client_tasks.push(Box::new(discord_task));
    }

    join_all(client_tasks).await;
    Ok(())
}

async fn load_config(path: impl AsRef<Path>) -> Result<AppConfig> {
    let config_str = read_to_string(path).await.context("failed to read config file")?;
    toml::from_str(&config_str).context("failed to parse config")
}

async fn initialize_natsuki(config: &AppConfig) -> Result<Natsuki> {
    let Some(assistant_identity) = config.assistant.identities.get(&config.assistant.identity) else {
        bail!("assistant identity {} not defined", config.assistant.identity);
    };

    let llm = create_llm(&config.llm).await?;
    let storage = create_storage(&config.storage).await?;
    let natsuki = Natsuki::new(assistant_identity, llm, storage).await?;
    Ok(natsuki)
}

async fn register_simple_functions(tool_config: &AppConfigTool, natsuki: &Natsuki) -> Result<()> {
    natsuki.add_simple_function(SelfInfo::new()).await;
    natsuki.add_simple_function(LocalInfo::new()?).await;

    if let Some(image_generator_config) = &tool_config.image_generator {
        natsuki
            .add_simple_function(ImageGenerator::new(image_generator_config)?)
            .await;
    }
    if let Some(get_illust_url_config) = &tool_config.get_illust_url {
        natsuki
            .add_simple_function(GetIllustUrl::new(get_illust_url_config).await?)
            .await;
    }
    if let Some(exchange_rate_config) = &tool_config.exchange_rate {
        natsuki
            .add_simple_function(ExchangeRate::new(exchange_rate_config).await?)
            .await;
    }

    Ok(())
}
