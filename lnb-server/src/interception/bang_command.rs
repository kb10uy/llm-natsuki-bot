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
        let user_text = {
            let Some(user_message) = incomplete.last_user() else {
                return Ok(InterceptionStatus::Continue);
            };
            let mut text_contents = user_message.contents.iter().filter_map(|c| match c {
                UserMessageContent::Text(t) => Some(t.as_str()),
                _ => None,
            });
            let Some(first_text_content) = text_contents.next() else {
                return Ok(InterceptionStatus::Continue);
            };
            first_text_content.trim()
        };
        let (command_name, rest) = {
            let Some(bang_stripped) = user_text.strip_prefix('!') else {
                return Ok(InterceptionStatus::Continue);
            };
            match bang_stripped.split_once(|c: char| c.is_whitespace()) {
                Some((c, r)) => (c, r),
                None => (bang_stripped, ""),
            }
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
