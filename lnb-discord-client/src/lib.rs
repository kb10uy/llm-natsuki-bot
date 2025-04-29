mod inner;
mod text;

use crate::inner::DiscordLnbClientInner;

use std::sync::Arc;

use futures::{future::BoxFuture, prelude::*};
use lnb_common::config::client::ConfigClientDiscord;
use lnb_core::{
    error::ClientError,
    interface::{client::LnbClient, server::LnbServer},
};

pub struct DiscordLnbClient<S>(Arc<inner::DiscordLnbClientInner<S>>);

impl<S: LnbServer> DiscordLnbClient<S> {
    pub async fn new(config: &ConfigClientDiscord, assistant: S) -> Result<DiscordLnbClient<S>, ClientError> {
        let inner_discord = DiscordLnbClientInner::new(config, assistant).await?;
        Ok(DiscordLnbClient(Arc::new(inner_discord)))
    }
}

impl<S: LnbServer> LnbClient for DiscordLnbClient<S> {
    fn execute(&self) -> BoxFuture<'static, Result<(), ClientError>> {
        let cloned = self.0.clone();
        async move {
            cloned
                .execute()
                .map_err(|e| ClientError::Communication(e.into()))
                .await?;
            Ok(())
        }
        .boxed()
    }
}
