use crate::{
    error::LlmError,
    interface::Context,
    model::{conversation::IncompleteConversation, message::AssistantMessage},
};

use futures::future::BoxFuture;

pub type BoxInterception = Box<dyn Interception + 'static>;

/// Llm に渡す前に処理を挟む。
pub trait Interception: Send + Sync {
    fn before_llm<'a>(
        &'a self,
        context: &'a Context,
        incomplete: &'a mut IncompleteConversation,
    ) -> BoxFuture<'a, Result<InterceptionStatus, LlmError>>;
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InterceptionStatus {
    /// 処理を続行する(後続の `Llm::send_conversation` が実行される)。
    #[default]
    Continue,

    /// 処理を続行する(Interception はスキップされるが後続の `Llm::send_conversation` が実行される)。
    Bypass,

    /// 処理を完了する(後続の `Llm::send_conversation` は実行されずすぐに `ConversationUpdate` が構築される)。
    Complete(AssistantMessage),

    /// 処理を中断する。
    Abort,
}

impl<T: Interception + 'static> From<T> for BoxInterception {
    fn from(value: T) -> BoxInterception {
        Box::new(value)
    }
}
