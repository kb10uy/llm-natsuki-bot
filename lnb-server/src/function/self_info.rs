use crate::{config::tools::ConfigToolsSelfInfo, function::ConfigurableFunction};

use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    context::Context,
    error::FunctionError,
    interface::{
        MessageContext,
        function::{Function, FunctionDescriptor, FunctionResponse},
    },
    model::{conversation::IncompleteConversation, message::MessageToolCalling, schema::DescribedSchema},
};
use lnb_rate_limiter::RateLimiter;
use serde_json::json;

#[derive(Debug)]
pub struct SelfInfo {
    descriptor: FunctionDescriptor,
}

impl ConfigurableFunction for SelfInfo {
    const NAME: &'static str = stringify!(SelfInfo);

    type Configuration = ConfigToolsSelfInfo;

    async fn configure(config: &ConfigToolsSelfInfo, _: Option<RateLimiter>) -> Result<SelfInfo, FunctionError> {
        Ok(SelfInfo {
            descriptor: FunctionDescriptor {
                name: "self_info".to_string(),
                description: config.prompt.description.clone(),
                parameters: DescribedSchema::object("parameters", "引数", vec![]),
            },
        })
    }
}

impl Function for SelfInfo {
    fn get_descriptor(&self) -> FunctionDescriptor {
        self.descriptor.clone()
    }

    fn call<'a>(
        &'a self,
        _ctx: &'a Context,
        _message_ctx: &'a MessageContext,
        _incomplete: &'a IncompleteConversation,
        _tool_calling: MessageToolCalling,
    ) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>> {
        async { self.get_info() }.boxed()
    }
}

impl SelfInfo {
    fn get_info(&self) -> Result<FunctionResponse, FunctionError> {
        Ok(FunctionResponse {
            result: json!({
                "bot_version": env!("CARGO_PKG_VERSION"),
                "bot_commit": env!("GIT_COMMIT_HASH"),
                "bot_binary_built_at": env!("BUILT_AT_DATETIME"),
            }),
            ..Default::default()
        })
    }
}
