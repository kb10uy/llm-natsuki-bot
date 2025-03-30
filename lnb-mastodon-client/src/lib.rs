mod inner;
mod text;

use crate::inner::MastodonLnbClientInner;

use std::sync::Arc;

use futures::{future::BoxFuture, prelude::*};
use lnb_core::{
    config::AppConfigClientMastodon,
    error::ClientError,
    interface::{client::LnbClient, server::LnbServer},
};

#[derive(Debug)]
pub struct MastodonLnbClient<S>(Arc<MastodonLnbClientInner<S>>);

impl<S: LnbServer> MastodonLnbClient<S> {
    pub async fn new(
        config_mastodon: &AppConfigClientMastodon,
        assistant: S,
    ) -> Result<MastodonLnbClient<S>, ClientError> {
        let inner = MastodonLnbClientInner::new(config_mastodon, assistant).await?;
        Ok(MastodonLnbClient(Arc::new(inner)))
    }
}

impl<S: LnbServer> LnbClient for MastodonLnbClient<S> {
    fn execute(&self) -> BoxFuture<'static, Result<(), ClientError>> {
        let cloned_inner = self.0.clone();
        async {
            cloned_inner.execute().await?;
            Ok(())
        }
        .boxed()
    }
}
