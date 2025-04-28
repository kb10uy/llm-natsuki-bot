mod bang_command;
mod cli;
mod function;
mod llm;
mod natsuki;
mod shiyu;
mod storage;

use crate::{
    bang_command::initialize_bang_command,
    function::{
        ConfigurableSimpleFunction, DailyPrivate, ExchangeRate, GetIllustUrl, ImageGenerator, LocalInfo, SelfInfo,
    },
    natsuki::{FunctionStore, LlmCache, Natsuki},
    shiyu::{Shiyu, ShiyuProvider},
    storage::initialize_storage,
};

use std::{collections::HashMap, path::Path, sync::Arc};

use anyhow::{Context as _, Result};
use clap::Parser;
use futures::future::{join, join_all};
use lnb_common::config::{Config, tools::ConfigTools};
use lnb_core::interface::{
    client::LnbClient,
    function::{complex::ArcComplexFunction, simple::ArcSimpleFunction},
    interception::BoxInterception,
};
use lnb_discord_client::DiscordLnbClient;
use lnb_mastodon_client::MastodonLnbClient;
use tokio::{fs::read_to_string, spawn};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = cli::Arguments::parse();
    let debug_options: HashMap<_, _> = args.debug_options.into_iter().collect();
    let config = load_config(args.config).await?;

    let (natsuki, shiyu) = initialize_natsuki(&config).await?;

    let mut client_tasks = vec![];

    // Mastodon
    if let Some(mastodon_config) = &config.client.mastodon {
        info!("starting Mastodon client");
        let mastodon_client = MastodonLnbClient::new(mastodon_config, &debug_options, natsuki.clone()).await?;
        shiyu.register_remindable(mastodon_client.clone()).await;

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

    let shiyu_task = shiyu.run(natsuki.clone());

    let (shiyu_result, client_results) = join(shiyu_task, join_all(client_tasks)).await;
    for client_join in client_results {
        let client_result = client_join?;
        client_result?;
    }
    shiyu_result?;

    Ok(())
}

async fn load_config(path: impl AsRef<Path>) -> Result<Config> {
    let config_str = read_to_string(path).await.context("failed to read config file")?;
    serde_json::from_str(&config_str).context("failed to parse config")
}

async fn initialize_natsuki(config: &Config) -> Result<(Natsuki, Shiyu)> {
    // Reminder
    let shiyu = Shiyu::new(&config.reminder).await?;
    let shiyu_provider = ShiyuProvider::new(&config.reminder, shiyu.clone()).await?;

    // Storage
    let storage = initialize_storage(&config.storage).await?;
    info!("using storage engine: {}", storage.description());

    // LlmCache
    let llm_cache = LlmCache::new(&config.llm);
    info!("{} LLM backend definitions loaded", config.llm.models.len());

    // Functions
    let simple_functions = initialize_simple_functions(&config.tools).await?;
    let complex_functions: Vec<ArcComplexFunction> = vec![Arc::new(shiyu_provider)];
    let function_store = FunctionStore::new(simple_functions, complex_functions);

    // Interceptions
    let interceptions = initialize_interceptions().await?;

    let natsuki = Natsuki::new(storage, llm_cache, function_store, interceptions, &config.assistant).await?;
    Ok((natsuki, shiyu))
}

async fn initialize_simple_functions(tool_config: &ConfigTools) -> Result<Vec<ArcSimpleFunction>> {
    let mut functions: Vec<ArcSimpleFunction> = vec![];

    functions.push(Arc::new(SelfInfo::new()));
    functions.push(Arc::new(LocalInfo::new()?));

    functions.extend(configure_simple_function::<ImageGenerator>(&tool_config.image_generator).await?);
    functions.extend(configure_simple_function::<ExchangeRate>(&tool_config.exchange_rate).await?);
    functions.extend(configure_simple_function::<GetIllustUrl>(&tool_config.get_illust_url).await?);
    functions.extend(configure_simple_function::<DailyPrivate>(&tool_config.daily_private).await?);

    Ok(functions)
}

async fn initialize_interceptions() -> Result<Vec<BoxInterception>> {
    Ok(vec![initialize_bang_command().await.into()])
}

async fn configure_simple_function<F>(config: &Option<F::Configuration>) -> Result<Option<ArcSimpleFunction>>
where
    F: ConfigurableSimpleFunction + 'static,
{
    let Some(config) = config.as_ref() else {
        return Ok(None);
    };

    let simple_function = F::configure(config).await?;
    info!("simple function configured: {}", F::NAME);
    Ok(Some(Arc::new(simple_function)))
}
