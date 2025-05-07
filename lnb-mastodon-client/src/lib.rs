mod inner;
mod text;

use crate::inner::MastodonLnbClientInner;

use std::sync::Arc;

use futures::{future::BoxFuture, prelude::*};
use lnb_common::{config::client::ConfigClientMastodon, user_roles::UserRolesGroup};
use lnb_core::{
    error::{ClientError, ReminderError},
    interface::{client::LnbClient, reminder::Remindable, server::LnbServer},
    model::conversation::ConversationUpdate,
};

const CONTEXT_KEY_PREFIX: &str = "mastodon";

#[derive(Debug, Clone)]
pub struct MastodonLnbClient<S>(Arc<MastodonLnbClientInner<S>>);

impl<S: LnbServer> MastodonLnbClient<S> {
    pub async fn new(
        config: &ConfigClientMastodon,
        roles_group: UserRolesGroup,
        assistant: S,
    ) -> Result<MastodonLnbClient<S>, ClientError> {
        let inner = MastodonLnbClientInner::new(config, roles_group, assistant).await?;
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
