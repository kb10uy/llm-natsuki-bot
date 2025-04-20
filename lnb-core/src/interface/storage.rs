use crate::{
    error::StorageError,
    model::conversation::{Conversation, ConversationId},
};

use futures::future::BoxFuture;

pub type BoxConversationStorage = Box<dyn ConversationStorage + 'static>;

/// `Conversation` の永続化層の抽象化。
/// 本当は Repository と Service に分けたりした方がいいんだろうけど、面倒なのでこれで……。
pub trait ConversationStorage: Send + Sync {
    fn description(&self) -> String;

    /// `ConversationId` から `Conversation` 本体を取得する。
    fn fetch_content_by_id(&self, id: ConversationId) -> BoxFuture<'_, Result<Option<Conversation>, StorageError>>;

    /// context key から `Conversation` 本体を取得する。
    fn fetch_content_by_context_key<'a>(
        &'a self,
        context_key: &'a str,
    ) -> BoxFuture<'a, Result<Option<Conversation>, StorageError>>;

    /// context key から `ConversationId` だけ取得する。
    fn fetch_id_by_context_key<'a>(
        &'a self,
        context_key: &'a str,
    ) -> BoxFuture<'a, Result<Option<ConversationId>, StorageError>>;

    /// `Conversation` を登録・更新する。
    fn upsert<'a>(
        &'a self,
        conversation: &'a Conversation,
        context_key: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), StorageError>>;
}

impl<T: ConversationStorage + 'static> From<T> for BoxConversationStorage {
    fn from(value: T) -> BoxConversationStorage {
        Box::new(value)
    }
}
