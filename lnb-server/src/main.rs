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
        ConfigurableFunction, DailyPrivate, ExchangeRate, GetIllustUrl, ImageGenerator, LocalInfo, MathRenderer,
        SelfInfo,
    },
    natsuki::{FunctionStore, LlmCache, Natsuki},
    shiyu::{Shiyu, ShiyuProvider},
    storage::initialize_storage,
};

use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, anyhow};
use clap::Parser;
use lnb_common::{
    config::{ConfigBot, load_bot_config, tools::ConfigTools},
    debug::set_debug_options,
    rate_limits::{RateLimits, RateLimitsCategory, load_rate_limits},
    user_roles::load_user_roles,
};
use lnb_core::interface::{client::LnbClient, function::ArcFunction, interception::BoxInterception};
use lnb_discord_client::DiscordLnbClient;
use lnb_mastodon_client::MastodonLnbClient;
use tokio::{signal, task::JoinSet};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = cli::Arguments::parse();
    let config = load_bot_config(args.config)?;
    let rate_limits = load_rate_limits(args.rate_limits)?;
    let user_roles = load_user_roles(args.user_roles)?;

    let debug_options: HashMap<_, _> = args.debug_options.into_iter().collect();
    set_debug_options(debug_options);

    let (natsuki, shiyu) = initialize_natsuki(&config, &rate_limits).await?;

    let mut services = JoinSet::new();

    // Mastodon
    if let Some(mastodon_config) = &config.client.mastodon {
        info!("starting Mastodon client");
        let mastodon_client = MastodonLnbClient::new(mastodon_config, user_roles.mastodon, natsuki.clone()).await?;
        shiyu.register_remindable(mastodon_client.clone()).await;

        services.spawn(async move {
            mastodon_client.execute().await?;
            Err(anyhow!("Mastodon client stopped unexpectedly"))
        });
    }

    // Discord
    if let Some(dicsord_config) = &config.client.discord {
        info!("starting Discord client");
        let discord_client = DiscordLnbClient::new(dicsord_config, user_roles.discord, natsuki.clone()).await?;

        services.spawn(async move {
            discord_client.execute().await?;
            Err(anyhow!("Discord client stopped unexpectedly"))
        });
    }

    services.spawn(async move {
        shiyu.run(natsuki).await?;
        Err(anyhow!("reminder service stopped unexpectedly"))
    });

    let result = tokio::select! {
        service = services.join_next() => match service {
            Some(Ok(result)) => result,
            Some(Err(join_error)) => Err(join_error.into()),
            None => Err(anyhow!("no services were started")),
        },
        signal_result = shutdown_signal() => {
            signal_result?;
            info!("shutdown signal received");
            Ok(())
        },
    };

    services.shutdown().await;
    result
}

async fn shutdown_signal() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut terminate = signal(SignalKind::terminate())?;
        tokio::select! {
            result = signal::ctrl_c() => result,
            _ = terminate.recv() => Ok(()),
        }
    }

    #[cfg(not(unix))]
    signal::ctrl_c().await
}

async fn initialize_natsuki(config: &ConfigBot, rate_limits: &RateLimits) -> Result<(Natsuki, Shiyu)> {
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
    let mut functions = initialize_functions(&config.tools, rate_limits).await?;
    functions.push(Arc::new(shiyu_provider));
    let function_store = FunctionStore::new(functions);

    // Interceptions
    let interceptions = initialize_interceptions().await?;

    let natsuki = Natsuki::new(
        storage,
        Some(rate_limits.conversation.clone().try_into()?),
        llm_cache,
        function_store,
        interceptions,
        &config.assistant,
    )
    .await?;
    Ok((natsuki, shiyu))
}

async fn initialize_functions(tool_config: &ConfigTools, rate_limits: &RateLimits) -> Result<Vec<ArcFunction>> {
    let mut functions: Vec<ArcFunction> = vec![];

    functions.push(Arc::new(SelfInfo::new()));
    functions.push(Arc::new(LocalInfo::new()?));

    functions.extend(
        configure_function::<ImageGenerator>(tool_config.image_generator.as_ref(), Some(&rate_limits.image_generator))
            .await?,
    );
    functions.extend(configure_function::<MathRenderer>(tool_config.math_renderer.as_ref(), None).await?);
    functions.extend(configure_function::<ExchangeRate>(tool_config.exchange_rate.as_ref(), None).await?);
    functions.extend(configure_function::<GetIllustUrl>(tool_config.get_illust_url.as_ref(), None).await?);
    functions.extend(configure_function::<DailyPrivate>(tool_config.daily_private.as_ref(), None).await?);

    Ok(functions)
}

async fn initialize_interceptions() -> Result<Vec<BoxInterception>> {
    Ok(vec![initialize_bang_command().await.into()])
}

async fn configure_function<F>(
    config: Option<&F::Configuration>,
    rate_limits_category: Option<&RateLimitsCategory>,
) -> Result<Option<ArcFunction>>
where
    F: ConfigurableFunction + 'static,
{
    let Some(config) = config else {
        return Ok(None);
    };
    let rate_limiter = rate_limits_category.cloned().map(TryInto::try_into).transpose();

    let simple_function = F::configure(config, rate_limiter?).await?;
    info!("simple function configured: {}", F::NAME);
    Ok(Some(Arc::new(simple_function)))
}
