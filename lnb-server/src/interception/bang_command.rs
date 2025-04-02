use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::LlmError,
    interface::interception::{Interception, InterceptionStatus},
    model::conversation::{IncompleteConversation, UserRole},
};

#[derive(Debug)]
pub struct BangCommandInterception {}

impl Interception for BangCommandInterception {
    fn before_llm<'a>(
        &'a self,
        incomplete: &'a mut IncompleteConversation,
        user_role: &'a UserRole,
    ) -> BoxFuture<'a, Result<InterceptionStatus, LlmError>> {
        async move { self.execute(incomplete, user_role).await }.boxed()
    }
}

impl BangCommandInterception {
    pub fn new() -> BangCommandInterception {
        BangCommandInterception {}
    }

    async fn execute(
        &self,
        _incomplete: &mut IncompleteConversation,
        _user_role: &UserRole,
    ) -> Result<InterceptionStatus, LlmError> {
        Ok(InterceptionStatus::Continue)
    }
}
