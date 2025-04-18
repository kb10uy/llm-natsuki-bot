mod inner;
mod text;

use crate::inner::MastodonLnbClientInner;

use std::{collections::HashMap, sync::Arc};

use futures::{future::BoxFuture, prelude::*};
use lnb_core::{
    DebugOptionValue,
    error::{ClientError, ReminderError},
    interface::{client::LnbClient, reminder::Remindable, server::LnbServer},
    model::conversation::ConversationUpdate,
};
use serde::Deserialize;

const CONTEXT_KEY_PREFIX: &str = "mastodon";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MastodonLnbClientConfig {
    pub server_url: String,
    pub token: String,
    pub sensitive_spoiler: String,
    pub max_length: usize,
    pub remote_fetch_delay_seconds: usize,
}

#[derive(Debug, Clone)]
pub struct MastodonLnbClient<S>(Arc<MastodonLnbClientInner<S>>);

impl<S: LnbServer> MastodonLnbClient<S> {
    pub async fn new(
        config: &MastodonLnbClientConfig,
        debug_options: &HashMap<String, DebugOptionValue>,
        assistant: S,
    ) -> Result<MastodonLnbClient<S>, ClientError> {
        let inner = MastodonLnbClientInner::new(config, debug_options, assistant).await?;
        Ok(MastodonLnbClient(Arc::new(inner)))
    }
}

impl<S: LnbServer> LnbClient for MastodonLnbClient<S> {
    fn execute(&self) -> BoxFuture<'static, Result<(), ClientError>> {
        let cloned_inner = self.0.clone();
        async move {
            cloned_inner.execute().await?;
            Ok(())
        }
        .boxed()
    }
}

impl<S: LnbServer> Remindable for MastodonLnbClient<S> {
    fn get_context(&self) -> String {
        CONTEXT_KEY_PREFIX.to_string()
    }

    fn remind(
        &self,
        requester: String,
        remind_conversation: ConversationUpdate,
    ) -> BoxFuture<'_, Result<(), ReminderError>> {
        async move {
            self.0
                .remind(requester, remind_conversation)
                .map_err(ReminderError::by_internal)
                .await
        }
        .boxed()
    }
}
