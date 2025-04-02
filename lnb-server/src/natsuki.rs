mod inner;

use crate::{config::AppConfigAssistantIdentity, natsuki::inner::NatsukiInner};

use std::sync::Arc;

use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::ServerError,
    interface::{
        function::simple::SimpleFunction, interception::BoxInterception, llm::BoxLlm, server::LnbServer,
        storage::BoxConversationStorage,
    },
    model::{
        conversation::{ConversationId, ConversationUpdate, UserRole},
        message::UserMessage,
    },
};

#[derive(Debug, Clone)]
pub struct Natsuki(Arc<NatsukiInner>);

impl Natsuki {
    pub async fn new(
        assistant_identity: &AppConfigAssistantIdentity,
        llm: BoxLlm,
        storage: BoxConversationStorage,
    ) -> Result<Natsuki, ServerError> {
        let inner = NatsukiInner::new(assistant_identity, llm, storage)?;
        Ok(Natsuki(Arc::new(inner)))
    }

    pub async fn add_simple_function(&self, simple_function: impl SimpleFunction + 'static) {
        self.0.add_simple_function(simple_function).await;
    }

    pub async fn apply_interception(&self, interception: impl Into<BoxInterception>) {
        self.0.apply_interception(interception.into()).await;
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
        conversation_id: ConversationId,
        user_message: UserMessage,
        user_role: UserRole,
    ) -> BoxFuture<'_, Result<ConversationUpdate, ServerError>> {
        async move {
            self.0
                .process_conversation(conversation_id, user_message, user_role)
                .await
        }
        .boxed()
    }
}
