use std::{collections::HashMap, sync::Arc};

use bimap::BiHashMap;
use futures::{FutureExt, future::BoxFuture};
use lnb_core::{error::StorageError, interface::storage::ConversationStorage, model::conversation::Conversation};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MemoryConversationStorage(Arc<MemoryConversationStorageInner>);

impl MemoryConversationStorage {
    pub fn new() -> MemoryConversationStorage {
        MemoryConversationStorage(Arc::new(MemoryConversationStorageInner {
            conversations: Mutex::new(HashMap::new()),
            platform_contexts: Mutex::new(BiHashMap::new()),
        }))
    }
}

impl ConversationStorage for MemoryConversationStorage {
    fn find_by_id<'a>(&'a self, id: &'a Uuid) -> BoxFuture<'a, Result<Option<Conversation>, StorageError>> {
        async move { self.0.find_by_id(id).await }.boxed()
    }

    fn find_by_platform_context<'a>(
        &'a self,
        platform: &'a str,
        context: &'a str,
    ) -> BoxFuture<'a, Result<Option<Conversation>, StorageError>> {
        async move { self.0.find_by_platform_context(platform, context).await }.boxed()
    }

    fn upsert<'a>(
        &'a self,
        conversation: &'a Conversation,
        platform: &'a str,
        new_context: &'a str,
    ) -> BoxFuture<'a, Result<(), StorageError>> {
        async move { self.0.upsert(conversation, platform, new_context).await }.boxed()
    }
}

#[derive(Debug)]
struct MemoryConversationStorageInner {
    conversations: Mutex<HashMap<Uuid, Conversation>>,
    platform_contexts: Mutex<BiHashMap<(String, String), Uuid>>,
}

impl MemoryConversationStorageInner {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Conversation>, StorageError> {
        let locked = self.conversations.lock().await;
        Ok(locked.get(id).cloned())
    }

    async fn find_by_platform_context(
        &self,
        platform: &str,
        context: &str,
    ) -> Result<Option<Conversation>, StorageError> {
        let locked_conv = self.conversations.lock().await;
        let locked_pc = self.platform_contexts.lock().await;

        let pc_key = (platform.to_string(), context.to_string()); // 本当に？
        let conversation = locked_pc
            .get_by_left(&pc_key)
            .and_then(|id| locked_conv.get(id).cloned());
        Ok(conversation)
    }

    async fn upsert(&self, conversation: &Conversation, platform: &str, new_context: &str) -> Result<(), StorageError> {
        let mut locked_conv = self.conversations.lock().await;
        let mut locked_pc = self.platform_contexts.lock().await;

        locked_conv.insert(conversation.id(), conversation.clone());
        locked_pc.remove_by_right(&conversation.id());
        locked_pc.insert((platform.to_string(), new_context.to_string()), conversation.id());
        Ok(())
    }
}
