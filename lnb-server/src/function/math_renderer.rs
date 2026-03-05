use crate::function::ConfigurableFunction;

use futures::{FutureExt, future::BoxFuture};
use lnb_common::{config::tools::ConfigToolsMathRenderer, math_renderer::MathRendererClient};
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
use lnb_rate_limiter::RateLimiter;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug)]
pub struct MathRenderer {
    renderer: MathRendererClient,
}

impl ConfigurableFunction for MathRenderer {
    const NAME: &'static str = stringify!(MathRenderer);

    type Configuration = ConfigToolsMathRenderer;

    async fn configure(
        config: &ConfigToolsMathRenderer,
        _rate_limits: Option<RateLimiter>,
    ) -> Result<MathRenderer, FunctionError> {
        let renderer = MathRendererClient::new(&config.endpoint, config.scale).map_err(FunctionError::by_external)?;
        Ok(MathRenderer { renderer })
    }
}

impl Function for MathRenderer {
    fn get_descriptor(&self) -> FunctionDescriptor {
        FunctionDescriptor {
            name: "math_renderer".to_string(),
            description: r#"
                ユーザーからの要望に基づき、プロンプトの入力から LaTeX 数式をレンダリングした画像を生成します。
                生成された画像は返答のメッセージに直接添付されます。
            "#
            .to_string(),
            parameters: DescribedSchema::object(
                "parameters",
                "引数",
                vec![
                    DescribedSchema::string("formula", "LaTeX 記法の数式。\\[ \\] や $ $ で囲む必要はありません。"),
                    DescribedSchema::boolean("display_mode", "数式をディスプレイモードでレンダリングするかどうか。"),
                ],
            ),
        }
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
