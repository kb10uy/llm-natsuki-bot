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

    fn insert<'a>(
        &'a self,
        conversation: &'a Conversation,
        context_key: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), StorageError>> {
        async move { self.0.insert(conversation, context_key).await }.boxed()
    }

    fn update_if_current<'a>(
        &'a self,
        expected: &'a Conversation,
        updated: &'a Conversation,
        context_key: &'a str,
    ) -> BoxFuture<'a, Result<bool, StorageError>> {
        async move { self.0.update_if_current(expected, updated, context_key).await }.boxed()
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

    async fn insert(&self, conversation: &Conversation, context_key: Option<&str>) -> Result<(), StorageError> {
        let mut locked_conv = self.conversations.lock().await;

        locked_conv.insert(conversation.id(), conversation.clone());

        if let Some(ck) = context_key {
            let mut locked_pc = self.context_keys.lock().await;
            locked_pc.remove_by_right(&conversation.id());
            locked_pc.insert(ck.to_string(), conversation.id());
        }
        Ok(())
    }

    async fn update_if_current(
        &self,
        expected: &Conversation,
        updated: &Conversation,
        context_key: &str,
    ) -> Result<bool, StorageError> {
        let mut locked_conv = self.conversations.lock().await;
        if locked_conv.get(&expected.id()) != Some(expected) {
            return Ok(false);
        }
        locked_conv.insert(updated.id(), updated.clone());

        let mut locked_pc = self.context_keys.lock().await;
        locked_pc.remove_by_right(&updated.id());
        locked_pc.insert(context_key.to_string(), updated.id());
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use lnb_core::model::{
        conversation::IncompleteConversation,
        message::{AssistantMessage, Message, UserMessageContent},
    };

    #[tokio::test]
    async fn stale_conversation_update_is_rejected() {
        let storage = MemoryConversationStorage::new();
        let original = Conversation::new_now(Some(Message::new_system("system")));
        storage.insert(&original, None).await.unwrap();

        let update = |text: &str| {
            let mut incomplete = IncompleteConversation::start(original.clone());
            incomplete.extend_messages([Message::new_user(
                [UserMessageContent::Text(text.to_string())],
                None,
                None,
                false,
            )]);
            incomplete.finish(AssistantMessage::default())
        };
        let first = update("first").complete_conversation_with(original.clone());
        let second = update("second").complete_conversation_with(original.clone());

        assert!(storage.update_if_current(&original, &first, "first").await.unwrap());
        assert!(!storage.update_if_current(&original, &second, "second").await.unwrap());
    }
}
