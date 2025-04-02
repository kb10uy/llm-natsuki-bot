use crate::{error::LlmError, model::conversation::IncompleteConversation};

use std::fmt::Debug;

use futures::future::BoxFuture;

/// Llm に渡す前に処理を挟む。
pub trait Interception: Send + Sync + Debug {
    fn before_llm<'a>(
        &'a self,
        incomplete: &'a mut IncompleteConversation,
    ) -> BoxFuture<'a, Result<InterceptionStatus, LlmError>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterceptionStatus {
    /// 処理を続行する(後続の `Llm::send_conversation` が実行される)。
    Continue,

    /// 処理を完了する(後続の `Llm::send_conversation` は実行されずすぐに `ConversationUpdate` が構築される)。
    Complete,

    /// 処理を中断する。
    Abort,
}
