use crate::function::ConfigurableFunction;

use futures::{FutureExt, future::BoxFuture};
use lnb_common::config::tools::ConfigToolsMathRenderer;
use lnb_core::{
    APP_USER_AGENT,
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
use reqwest::{Client as ReqwestClient, ClientBuilder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error as ThisError;
use url::Url;

#[derive(Debug)]
pub struct MathRenderer {
    http_client: ReqwestClient,
    render_endpoint: Url,
    scale: f64,
}

impl ConfigurableFunction for MathRenderer {
    const NAME: &'static str = stringify!(MathRenderer);

    type Configuration = ConfigToolsMathRenderer;

    async fn configure(
        config: &ConfigToolsMathRenderer,
        _rate_limits: Option<RateLimiter>,
    ) -> Result<MathRenderer, FunctionError> {
        let http_client = ClientBuilder::new()
            .user_agent(APP_USER_AGENT)
            .build()
            .map_err(FunctionError::by_external)?;
        let render_endpoint = Url::parse(&config.endpoint).map_err(FunctionError::by_serialization)?;

        Ok(MathRenderer {
            http_client,
            render_endpoint,
            scale: config.scale,
        })
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
        async move {
            match self.execute(parameters).await {
                Ok(response) => Ok(response),
                Err(IntermediateError::AsResponse(message)) => Ok(FunctionResponse {
                    result: serde_json::to_value(RenderingError {
                        error: message.to_string(),
                    })
                    .map_err(FunctionError::by_serialization)?,
                    ..Default::default()
                }),
                Err(IntermediateError::Unrecoverable(err)) => Err(err),
            }
        }
        .boxed()
    }
}

impl MathRenderer {
    async fn execute(&self, parameters: RenderingParameters) -> Result<FunctionResponse, IntermediateError> {
        let payload = json!({
            "formula": parameters.formula,
            "display": parameters.display_mode,
            "scale": self.scale,
        });
        let resp = self
            .http_client
            .post(self.render_endpoint.clone())
            .json(&payload)
            .send()
            .await
            .map_err(|err| IntermediateError::AsResponse(err.to_string()))?;
        let png_bytes = if resp.status().is_success() {
            resp.bytes()
                .await
                .map_err(|err| IntermediateError::AsResponse(err.to_string()))?
        } else {
            let error: RenderingError = resp
                .json()
                .await
                .map_err(|err| IntermediateError::AsResponse(err.to_string()))?;
            return Err(IntermediateError::AsResponse(error.error));
        };

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RenderingError {
    error: String,
}

#[derive(Debug, ThisError)]
enum IntermediateError {
    #[error("as response error: {0}")]
    AsResponse(String),

    #[error("unrecoverable function error: {0}")]
    Unrecoverable(#[from] FunctionError),
}
