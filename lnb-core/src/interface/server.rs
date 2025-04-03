use futures::future::BoxFuture;

use crate::{
    error::ServerError,
    interface::Context,
    model::{
        conversation::{ConversationId, ConversationUpdate, UserRole},
        message::UserMessage,
    },
};

/// 旧 Assistant
pub trait LnbServer: Send + Sync + 'static {
    /// 新しい会話ツリーを開始する。
    fn new_conversation(&self) -> BoxFuture<'_, Result<ConversationId, ServerError>>;

    /// 会話ツリーを復元する。
    fn restore_conversation<'a>(
        &'a self,
        context_key: &'a str,
    ) -> BoxFuture<'a, Result<Option<ConversationId>, ServerError>>;

    /// 会話ツリーを更新する。
    fn save_conversation<'a>(
        &'a self,
        update: ConversationUpdate,
        context_key: &'a str,
    ) -> BoxFuture<'a, Result<(), ServerError>>;

    fn process_conversation(
        &self,
        context: Context,
        conversation_id: ConversationId,
        user_message: UserMessage,
        user_role: UserRole,
    ) -> BoxFuture<'_, Result<ConversationUpdate, ServerError>>;
}
