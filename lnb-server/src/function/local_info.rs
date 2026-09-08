use crate::{config::tools::ConfigToolsLocalInfo, function::ConfigurableFunction};

use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    RFC3339_NUMOFFSET,
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
use time::OffsetDateTime;

#[derive(Debug)]
pub struct LocalInfo {
    started_at: OffsetDateTime,
    descriptor: FunctionDescriptor,
}

impl ConfigurableFunction for LocalInfo {
    const NAME: &'static str = stringify!(LocalInfo);

    type Configuration = ConfigToolsLocalInfo;

    async fn configure(config: &ConfigToolsLocalInfo, _: Option<RateLimiter>) -> Result<LocalInfo, FunctionError> {
        Ok(LocalInfo {
            started_at: OffsetDateTime::now_local().map_err(FunctionError::by_external)?,
            descriptor: FunctionDescriptor {
                name: "local_info".to_string(),
                description: config.prompt.description.clone(),
                parameters: DescribedSchema::object("parameters", "引数", vec![]),
            },
        })
    }
}

impl Function for LocalInfo {
    fn get_descriptor(&self) -> FunctionDescriptor {
        self.descriptor.clone()
    }

    fn call<'a>(
        &'a self,
        ctx: &'a Context,
        _message_ctx: &'a MessageContext,
        _incomplete: &'a IncompleteConversation,
        _tool_calling: MessageToolCalling,
    ) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>> {
        async { self.get_info(ctx.datetime_provider.now()) }.boxed()
    }
}

impl LocalInfo {
    fn get_info(&self, now: OffsetDateTime) -> Result<FunctionResponse, FunctionError> {
        Ok(FunctionResponse {
            result: json!({
                "time_now": now.format(RFC3339_NUMOFFSET).map_err(FunctionError::by_serialization)?,
                "bot_started_at": self.started_at.format(RFC3339_NUMOFFSET).map_err(FunctionError::by_serialization)?,
            }),
            ..Default::default()
        })
    }
}
