mod bang_command;
mod cli;
mod config;
mod function;
mod llm;
mod natsuki;
mod storage;

use crate::{
    bang_command::initialize_bang_command,
    config::AppConfig,
    function::{ConfigurableFunction, ExchangeRate, GetIllustUrl, ImageGenerator, LocalInfo, SelfInfo},
    llm::initialize_llm,
    natsuki::Natsuki,
    storage::initialize_storage,
};

use std::path::Path;

use anyhow::{Context as _, Result, bail};
use clap::Parser;
use config::AppConfigTool;
use futures::future::join_all;
use lnb_core::interface::{client::LnbClient, function::simple::SimpleFunction};
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
    register_interceptions(&natsuki).await?;

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
    info!("using assistant identity: {}", config.assistant.identity);

    let (llm, llm_name) = initialize_llm(&config.llm).await?;
    let (storage, storage_name) = initialize_storage(&config.storage).await?;
    let natsuki = Natsuki::new(assistant_identity, llm, storage).await?;
    info!("assistant engine initialized (LLM engine: {llm_name}, storage engine: {storage_name})");

    Ok(natsuki)
}

async fn register_simple_functions(tool_config: &AppConfigTool, natsuki: &Natsuki) -> Result<()> {
    natsuki.add_simple_function(SelfInfo::new()).await;
    natsuki.add_simple_function(LocalInfo::new()?).await;

    register_simple_function_config::<ImageGenerator>(&tool_config.image_generator, natsuki).await?;
    register_simple_function_config::<GetIllustUrl>(&tool_config.get_illust_url, natsuki).await?;
    register_simple_function_config::<ExchangeRate>(&tool_config.exchange_rate, natsuki).await?;

    Ok(())
}

async fn register_interceptions(natsuki: &Natsuki) -> Result<()> {
    let bang_command = initialize_bang_command().await;
    natsuki.apply_interception(bang_command).await;
    Ok(())
}

async fn register_simple_function_config<F>(config: &Option<F::Configuration>, natsuki: &Natsuki) -> Result<()>
where
    F: SimpleFunction + ConfigurableFunction + 'static,
{
    let Some(config) = config.as_ref() else {
        return Ok(());
    };

    let simple_function = F::create(config).await?;
    natsuki.add_simple_function(simple_function).await;
    info!("simple function configured: {}", F::NAME);

    Ok(())
}
