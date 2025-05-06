mod function_store;
mod inner;
mod llm_cache;

pub use function_store::FunctionStore;
pub use llm_cache::LlmCache;
use lnb_common::config::assistant::ConfigAssistant;
use lnb_rate_limiter::RateLimiter;

use crate::natsuki::inner::NatsukiInner;

use std::sync::Arc;

use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::ServerError,
    interface::{Context, interception::BoxInterception, server::LnbServer, storage::BoxConversationStorage},
    model::{
        conversation::{ConversationId, ConversationUpdate},
        message::Message,
    },
};

#[derive(Clone)]
pub struct Natsuki(Arc<NatsukiInner>);

impl Natsuki {
    pub async fn new(
        storage: BoxConversationStorage,
        rate_limiter: Option<RateLimiter>,
        llm_cache: llm_cache::LlmCache,
        function_store: function_store::FunctionStore,
        interceptions: impl IntoIterator<Item = BoxInterception>,
        assistant_identity: &ConfigAssistant,
    ) -> Result<Natsuki, ServerError> {
        let inner = NatsukiInner::new(
            storage,
            rate_limiter,
            llm_cache,
            function_store,
            interceptions.into_iter().collect(),
            assistant_identity,
        )?;
        Ok(Natsuki(Arc::new(inner)))
    }
}

impl LnbServer for Natsuki {
    fn new_conversation(&self) -> BoxFuture<'_, Result<ConversationId, ServerError>> {
        async move { self.0.new_conversation().await }.boxed()
    }

    fn restore_conversation<'a>(
        &'a self,
        context_key: &'a str,
    ) -> BoxFuture<'a, Result<Option<ConversationId>, ServerError>> {
        async move { self.0.restore_conversation(context_key).await }.boxed()
    }

    fn save_conversation<'a>(
        &'a self,
        update: ConversationUpdate,
        context_key: &'a str,
    ) -> BoxFuture<'a, Result<(), ServerError>> {
        async move { self.0.save_conversation(update, context_key).await }.boxed()
    }

    fn process_conversation(
        &self,
        context: Context,
        conversation_id: ConversationId,
        user_message: Vec<Message>,
    ) -> BoxFuture<'_, Result<ConversationUpdate, ServerError>> {
        async move {
            self.0
                .process_conversation(context, conversation_id, user_message)
                .await
        }
        .boxed()
    }
}
