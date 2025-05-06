use std::collections::HashMap;

use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::LlmError,
    interface::{
        Context,
        interception::{Interception, InterceptionStatus},
    },
    model::{
        conversation::{ConversationModel, IncompleteConversation},
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
    ) -> BoxFuture<'a, Result<InterceptionStatus, LlmError>> {
        async move { self.execute(context, incomplete).await }.boxed()
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
    ) -> Result<InterceptionStatus, LlmError> {
        let Some(last_user_message) = incomplete.last_user_mut() else {
            return Ok(InterceptionStatus::Continue);
        };

        let user_text = {
            let mut text_contents = last_user_message.contents.iter().filter_map(|c| match c {
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
        last_user_message.skip_llm = true;

        let commands = self.commands.read().await;
        let Some(command) = commands.get(command_name) else {
            return Ok(self.complete_with(format!("unknown command: {command_name}")));
        };

        let result_status = command.call(context, rest).await?;
        if let Some(model_override) = result_status.model_override {
            incomplete.set_model_override(model_override);
        }
        Ok(result_status.status)
    }

    fn complete_with(&self, text: impl Into<String>) -> InterceptionStatus {
        InterceptionStatus::Complete(AssistantMessage {
            text: text.into(),
            skip_llm: true,
            ..Default::default()
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct BangCommandResponse {
    pub status: InterceptionStatus,
    pub model_override: Option<ConversationModel>,
}

/// `BangCommandInterception` から呼び出されるコマンドの実装。
pub trait BangCommand: Send + Sync {
    fn call<'a>(
        &'a self,
        context: &'a Context,
        rest_text: &'a str,
    ) -> BoxFuture<'a, Result<BangCommandResponse, LlmError>>;
}

impl<T: BangCommand + 'static> From<T> for BoxBangCommand {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

type BoxedAsyncClosure =
    Box<dyn Send + Sync + for<'a> Fn(&'a Context, &'a str) -> BoxFuture<'a, Result<BangCommandResponse, LlmError>>>;
type BoxedSyncClosure =
    Box<dyn Send + Sync + for<'a> Fn(&'a Context, &'a str) -> Result<BangCommandResponse, LlmError>>;

pub struct AsyncClosure(BoxedAsyncClosure);
pub struct SyncClosure(BoxedSyncClosure);

impl BangCommand for AsyncClosure {
    fn call<'a>(
        &'a self,
        context: &'a Context,
        rest_text: &'a str,
    ) -> BoxFuture<'a, Result<BangCommandResponse, LlmError>> {
        (self.0)(context, rest_text)
    }
}

impl BangCommand for SyncClosure {
    fn call<'a>(
        &'a self,
        context: &'a Context,
        rest_text: &'a str,
    ) -> BoxFuture<'a, Result<BangCommandResponse, LlmError>> {
        async { (self.0)(context, rest_text) }.boxed()
    }
}

#[allow(dead_code)]
pub fn async_fn_command<F>(f: F) -> AsyncClosure
where
    F: Send + Sync + for<'a> Fn(&'a Context, &'a str) -> BoxFuture<'a, Result<BangCommandResponse, LlmError>> + 'static,
{
    AsyncClosure(Box::new(f))
}

#[allow(dead_code)]
pub fn fn_command<F>(f: F) -> SyncClosure
where
    F: Send + Sync + for<'a> Fn(&'a Context, &'a str) -> Result<BangCommandResponse, LlmError> + 'static,
{
    SyncClosure(Box::new(f))
}
