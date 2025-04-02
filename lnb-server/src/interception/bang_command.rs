use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::LlmError,
    interface::interception::{Interception, InterceptionStatus},
    model::{
        conversation::{IncompleteConversation, UserRole},
        message::{AssistantMessage, UserMessageContent},
    },
};
use tracing::debug;

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
        incomplete: &mut IncompleteConversation,
        _user_role: &UserRole,
    ) -> Result<InterceptionStatus, LlmError> {
        let user_text = incomplete
            .last_user()
            .ok_or_else(|| LlmError::ExpectationMismatch("user message not found".to_string()))?
            .contents
            .iter()
            .filter_map(|c| match c {
                UserMessageContent::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .next()
            .ok_or_else(|| LlmError::ExpectationMismatch("text content not found".to_string()))?
            .trim();

        let (command_name, rest) = {
            let Some(bang_stripped) = user_text.strip_prefix('!') else {
                return Ok(InterceptionStatus::Continue);
            };
            let Some((command_name, rest)) = bang_stripped.split_once(|c: char| c.is_whitespace()) else {
                return Ok(InterceptionStatus::Continue);
            };
            (command_name, rest.trim())
        };
        debug!("bang command: {command_name} [{rest}]");

        Ok(self.complete_with(format!("unknown command: {command_name}")))
    }

    fn complete_with(&self, text: impl Into<String>) -> InterceptionStatus {
        InterceptionStatus::Complete(AssistantMessage {
            text: text.into(),
            ..Default::default()
        })
    }
}
