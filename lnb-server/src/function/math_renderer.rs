use crate::function::ConfigurableFunction;

use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    context::Context,
    error::FunctionError,
    interface::{
        MessageContext,
        function::{Function, FunctionDescriptor, FunctionResponse},
    },
    model::{
        conversation::{ConversationAttachment, IncompleteConversation},
        message::MessageToolCalling,
        schema::DescribedSchema,
    },
};
use lnb_math_renderer_client::MathRendererClient;
use lnb_rate_limiter::RateLimiter;
use serde::Deserialize;
use serde_json::json;

use crate::config::tools::ConfigToolsMathRenderer;

#[derive(Debug)]
pub struct MathRenderer {
    renderer: MathRendererClient,
    descriptor: FunctionDescriptor,
}

impl ConfigurableFunction for MathRenderer {
    const NAME: &'static str = stringify!(MathRenderer);

    type Configuration = ConfigToolsMathRenderer;

    async fn configure(
        config: &ConfigToolsMathRenderer,
        _rate_limits: Option<RateLimiter>,
    ) -> Result<MathRenderer, FunctionError> {
        let renderer = MathRendererClient::new(&config.endpoint, config.scale).map_err(FunctionError::by_external)?;
        let descriptor = FunctionDescriptor {
            name: "math_renderer".to_string(),
            description: config.prompt.description.clone(),
            parameters: DescribedSchema::object(
                "parameters",
                "引数",
                vec![
                    DescribedSchema::string("formula", config.prompt.parameter("formula")?),
                    DescribedSchema::boolean("display_mode", config.prompt.parameter("display_mode")?),
                ],
            ),
        };
        Ok(MathRenderer { renderer, descriptor })
    }
}

impl Function for MathRenderer {
    fn get_descriptor(&self) -> FunctionDescriptor {
        self.descriptor.clone()
    }

    fn call<'a>(
        &'a self,
        _ctx: &'a Context,
        _message_ctx: &'a MessageContext,
        _incomplete: &'a IncompleteConversation,
        tool_calling: MessageToolCalling,
    ) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>> {
        let parameters = match serde_json::from_value(tool_calling.arguments).map_err(FunctionError::by_serialization) {
            Ok(p) => p,
            Err(err) => return async { Err(FunctionError::Serialization(err.into())) }.boxed(),
        };
        async move { self.execute(parameters).await }.boxed()
    }
}

impl MathRenderer {
    async fn execute(&self, parameters: RenderingParameters) -> Result<FunctionResponse, FunctionError> {
        let png_bytes = self
            .renderer
            .render(&parameters.formula, parameters.display_mode)
            .await
            .map_err(FunctionError::by_external)?;

        let image_attachment = ConversationAttachment::Image {
            bytes: png_bytes.to_vec(),
            description: Some(parameters.formula),
        };

        Ok(FunctionResponse {
            result: json!({
                "status": "success",
            }),
            attachments: vec![image_attachment],
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RenderingParameters {
    formula: String,
    display_mode: bool,
}
