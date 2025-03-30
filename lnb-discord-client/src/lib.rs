mod inner;
mod text;

use crate::inner::DiscordLnbClientInner;

use std::sync::Arc;

use futures::{future::BoxFuture, prelude::*};
use lnb_core::{
    error::ClientError,
    interface::{client::LnbClient, server::LnbServer},
};
use serde::Deserialize;
use serenity::Client as SerenityClient;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DiscordLnbClientConfig {
    pub token: String,
    pub max_length: usize,
}

pub struct DiscordLnbClient(Arc<Mutex<SerenityClient>>);

impl DiscordLnbClient {
    pub async fn new(
        config: &DiscordLnbClientConfig,
        assistant: impl LnbServer,
    ) -> Result<DiscordLnbClient, ClientError> {
        let inner_discord = DiscordLnbClientInner::new_as_serenity_client(config, assistant).await?;
        Ok(DiscordLnbClient(Arc::new(Mutex::new(inner_discord))))
    }
}

impl LnbClient for DiscordLnbClient {
    fn execute(&self) -> BoxFuture<'static, Result<(), ClientError>> {
        let cloned = self.0.clone();
        async move {
            let mut locked = cloned.lock().await;
            locked.start().map_err(|e| ClientError::Communication(e.into())).await?;
            Ok(())
        }
        .boxed()
    }
}
