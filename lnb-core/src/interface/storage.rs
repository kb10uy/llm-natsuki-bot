use crate::{
    error::StorageError,
    model::conversation::{Conversation, ConversationId},
};

use std::fmt::Debug;

use futures::future::BoxFuture;

/// `Conversation` の永続化層の抽象化。
/// 本当は Repository と Service に分けたりした方がいいんだろうけど、面倒なのでこれで……。
#[allow(dead_code)]
pub trait ConversationStorage: Send + Sync + Debug {
    /// `ConversationId` から `Conversation` 本体を取得する。
    fn fetch_content_by_id(&self, id: ConversationId) -> BoxFuture<'_, Result<Option<Conversation>, StorageError>>;

    /// context key から `Conversation` 本体を取得する。
    fn fetch_content_by_context_key<'a>(
        &'a self,
        context_key: &'a str,
    ) -> BoxFuture<'a, Result<Option<Conversation>, StorageError>>;

    /// `Conversation` を登録・更新する。
    fn upsert<'a>(
        &'a self,
        conversation: &'a Conversation,
        context_key: &'a str,
    ) -> BoxFuture<'a, Result<(), StorageError>>;
}
