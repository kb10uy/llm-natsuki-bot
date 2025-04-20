use std::{collections::HashMap, sync::Arc};

use bimap::BiHashMap;
use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::StorageError,
    interface::storage::ConversationStorage,
    model::conversation::{Conversation, ConversationId},
};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct MemoryConversationStorage(Arc<MemoryConversationStorageInner>);

impl MemoryConversationStorage {
    pub fn new() -> MemoryConversationStorage {
        MemoryConversationStorage(Arc::new(MemoryConversationStorageInner {
            conversations: Mutex::new(HashMap::new()),
            context_keys: Mutex::new(BiHashMap::new()),
        }))
    }
}

impl ConversationStorage for MemoryConversationStorage {
    fn description(&self) -> String {
        "HashMap Memory".to_string()
    }

    fn fetch_content_by_id(&self, id: ConversationId) -> BoxFuture<'_, Result<Option<Conversation>, StorageError>> {
        async move { self.0.fetch_content_by_id(id).await }.boxed()
    }

    fn fetch_content_by_context_key<'a>(
        &'a self,
        context_key: &'a str,
    ) -> BoxFuture<'a, Result<Option<Conversation>, StorageError>> {
        async move { self.0.fetch_content_by_context_key(context_key).await }.boxed()
    }

    fn fetch_id_by_context_key<'a>(
        &'a self,
        context_key: &'a str,
    ) -> BoxFuture<'a, Result<Option<ConversationId>, StorageError>> {
        async move { self.0.fetch_id_by_context_key(context_key).await }.boxed()
    }

    fn upsert<'a>(
        &'a self,
        conversation: &'a Conversation,
        context_key: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), StorageError>> {
        async move { self.0.upsert(conversation, context_key).await }.boxed()
    }
}

#[derive(Debug)]
struct MemoryConversationStorageInner {
    conversations: Mutex<HashMap<ConversationId, Conversation>>,
    context_keys: Mutex<BiHashMap<String, ConversationId>>,
}

impl MemoryConversationStorageInner {
    async fn fetch_content_by_id(&self, id: ConversationId) -> Result<Option<Conversation>, StorageError> {
        let locked = self.conversations.lock().await;
        Ok(locked.get(&id).cloned())
    }

    async fn fetch_content_by_context_key(&self, context_key: &str) -> Result<Option<Conversation>, StorageError> {
        let locked_conv = self.conversations.lock().await;
        let locked_pc = self.context_keys.lock().await;

        let conversation = locked_pc
            .get_by_left(context_key)
            .and_then(|id| locked_conv.get(id).cloned());
        Ok(conversation)
    }

    async fn fetch_id_by_context_key(&self, context_key: &str) -> Result<Option<ConversationId>, StorageError> {
        let locked_pc = self.context_keys.lock().await;
        let conversation_id = locked_pc.get_by_left(context_key).cloned();
        Ok(conversation_id)
    }

    async fn upsert(&self, conversation: &Conversation, context_key: Option<&str>) -> Result<(), StorageError> {
        let mut locked_conv = self.conversations.lock().await;

        locked_conv.insert(conversation.id(), conversation.clone());

        if let Some(ck) = context_key {
            let mut locked_pc = self.context_keys.lock().await;
            locked_pc.remove_by_right(&conversation.id());
            locked_pc.insert(ck.to_string(), conversation.id());
        }
        Ok(())
    }
}
