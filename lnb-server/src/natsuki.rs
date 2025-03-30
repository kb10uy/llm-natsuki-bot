mod inner;

use crate::{config::AppConfigAssistantIdentity, natsuki::inner::NatsukiInner};

use std::sync::Arc;

use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::ServerError,
    interface::{function::simple::SimpleFunction, llm::Llm, server::LnbServer, storage::ConversationStorage},
    model::{
        conversation::{Conversation, ConversationUpdate},
        message::UserMessage,
    },
};

#[derive(Debug, Clone)]
pub struct Natsuki(Arc<NatsukiInner>);

impl Natsuki {
    pub async fn new(
        assistant_identity: &AppConfigAssistantIdentity,
        llm: Box<dyn Llm + 'static>,
        storage: Box<dyn ConversationStorage + 'static>,
    ) -> Result<Natsuki, ServerError> {
        let inner = NatsukiInner::new(assistant_identity, llm, storage)?;
        Ok(Natsuki(Arc::new(inner)))
    }

    pub async fn add_simple_function(&self, simple_function: impl SimpleFunction + 'static) {
        self.0.add_simple_function(simple_function).await;
    }
}

impl LnbServer for Natsuki {
    fn new_conversation(&self) -> Conversation {
        self.0.new_conversation()
    }

    fn restore_conversation<'a>(
        &'a self,
        platform: &'a str,
        context: &'a str,
    ) -> BoxFuture<'a, Result<Option<Conversation>, ServerError>> {
        async move { self.0.restore_conversation(platform, context).await }.boxed()
    }

    fn save_conversation<'a>(
        &'a self,
        conversation: &'a Conversation,
        platform: &'a str,
        context: &'a str,
    ) -> BoxFuture<'a, Result<(), ServerError>> {
        async move { self.0.save_conversation(conversation, platform, context).await }.boxed()
    }

    fn process_conversation(
        &self,
        conversation: Conversation,
        user_message: UserMessage,
    ) -> BoxFuture<'_, Result<ConversationUpdate, ServerError>> {
        async move { self.0.process_conversation(conversation, user_message).await }.boxed()
    }
}
