use std::collections::HashMap;

use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::LlmError,
    interface::{
        Context,
        interception::{Interception, InterceptionStatus},
    },
    model::{
        conversation::{IncompleteConversation, UserRole},
        message::{AssistantMessage, UserMessageContent},
    },
};
use tokio::sync::RwLock;
use tracing::debug;

type BoxBangCommand = Box<dyn BangCommand + 'static>;

pub struct BangCommandInterception {
    commands: RwLock<HashMap<String, BoxBangCommand>>,
}

impl Interception for BangCommandInterception {
    fn before_llm<'a>(
        &'a self,
        context: &'a Context,
        incomplete: &'a mut IncompleteConversation,
        user_role: &'a UserRole,
    ) -> BoxFuture<'a, Result<InterceptionStatus, LlmError>> {
        async move { self.execute(context, incomplete, user_role).await }.boxed()
    }
}

impl BangCommandInterception {
    pub fn new() -> BangCommandInterception {
        BangCommandInterception {
            commands: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register_command(&self, name: impl Into<String>, command: impl Into<BoxBangCommand>) {
        let mut locked = self.commands.write().await;
        locked.insert(name.into(), command.into());
    }

    async fn execute(
        &self,
        context: &Context,
        incomplete: &mut IncompleteConversation,
        user_role: &UserRole,
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

        let commands = self.commands.read().await;
        let Some(command) = commands.get(command_name) else {
            return Ok(self.complete_with(format!("unknown command: {command_name}")));
        };

        let result_message = command.call(context, rest, user_role).await?;
        Ok(InterceptionStatus::Complete(result_message))
    }

    fn complete_with(&self, text: impl Into<String>) -> InterceptionStatus {
        InterceptionStatus::Complete(AssistantMessage {
            text: text.into(),
            ..Default::default()
        })
    }
}

/// `BangCommandInterception` から呼び出されるコマンドの実装。
pub trait BangCommand: Send + Sync {
    fn call<'a>(
        &'a self,
        context: &'a Context,
        rest_text: &'a str,
        user_role: &'a UserRole,
    ) -> BoxFuture<'a, Result<AssistantMessage, LlmError>>;
}

impl<T: BangCommand + 'static> From<T> for BoxBangCommand {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

type BoxedAsyncClosure = Box<
    dyn Send
        + Sync
        + for<'a> Fn(&'a Context, &'a str, &'a UserRole) -> BoxFuture<'a, Result<AssistantMessage, LlmError>>,
>;
type BoxedSyncClosure =
    Box<dyn Send + Sync + for<'a> Fn(&'a Context, &'a str, &'a UserRole) -> Result<AssistantMessage, LlmError>>;

pub struct AsyncClosure(BoxedAsyncClosure);
pub struct SyncClosure(BoxedSyncClosure);

impl BangCommand for AsyncClosure {
    fn call<'a>(
        &'a self,
        context: &'a Context,
        rest_text: &'a str,
        user_role: &'a UserRole,
    ) -> BoxFuture<'a, Result<AssistantMessage, LlmError>> {
        (self.0)(context, rest_text, user_role)
    }
}

impl BangCommand for SyncClosure {
    fn call<'a>(
        &'a self,
        context: &'a Context,
        rest_text: &'a str,
        user_role: &'a UserRole,
    ) -> BoxFuture<'a, Result<AssistantMessage, LlmError>> {
        async { (self.0)(context, rest_text, user_role) }.boxed()
    }
}

pub fn async_fn_command<F>(f: F) -> AsyncClosure
where
    F: Send
        + Sync
        + for<'a> Fn(&'a Context, &'a str, &'a UserRole) -> BoxFuture<'a, Result<AssistantMessage, LlmError>>
        + 'static,
{
    AsyncClosure(Box::new(f))
}

pub fn fn_command<F>(f: F) -> SyncClosure
where
    F: Send + Sync + for<'a> Fn(&'a Context, &'a str, &'a UserRole) -> Result<AssistantMessage, LlmError> + 'static,
{
    SyncClosure(Box::new(f))
}
