use crate::ConfigurableSimpleFunction;

use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{CreateImageRequest, Image, ImageModel},
};
use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    APP_USER_AGENT,
    error::FunctionError,
    interface::function::{FunctionDescriptor, FunctionResponse, simple::SimpleFunction},
    model::{conversation::ConversationAttachment, schema::DescribedSchema},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
pub struct ImageGeneratorConfig {
    pub endpoint: String,
    pub token: String,
    pub model: String,
}

#[derive(Debug)]
pub struct ImageGenerator {
    client: Client<OpenAIConfig>,
    model: String,
}

impl ConfigurableSimpleFunction for ImageGenerator {
    const NAME: &'static str = stringify!(ImageGenerator);

    type Configuration = ImageGeneratorConfig;

    async fn configure(config: &ImageGeneratorConfig) -> Result<ImageGenerator, FunctionError> {
        let openai_config = OpenAIConfig::new()
            .with_api_key(&config.token)
            .with_api_base(&config.endpoint);
        let http_client = reqwest::ClientBuilder::new()
            .user_agent(APP_USER_AGENT)
            .build()
            .map_err(|e| FunctionError::External(e.into()))?;

        let client = Client::with_config(openai_config).with_http_client(http_client);
        Ok(ImageGenerator {
            client,
            model: config.model.to_string(),
        })
    }
}

impl SimpleFunction for ImageGenerator {
    fn get_descriptor(&self) -> FunctionDescriptor {
        FunctionDescriptor {
            name: "image_generator".to_string(),
            description: r#"
                ユーザーからの要望に基づき、プロンプトの入力から AI を利用して画像を生成します。生成された画像の URL は返答文に含めないでください。
                ユーザーから明示的に画像生成の要求があるとき以外は決してこのツールを呼ばないでください。
            "#
            .to_string(),
            parameters: DescribedSchema::object(
                "parameters",
                "引数",
                vec![DescribedSchema::string(
                    "prompt",
                    "DALL-E 3 などの画像生成モデルに入力するプロンプト文。",
                )],
            ),
        }
    }

    fn call<'a>(&'a self, _id: &str, params: Value) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>> {
        let prompt = params["prompt"].as_str().unwrap_or_default().to_string();
        async move { self.generate(prompt.to_string()).await }.boxed()
    }
}

impl ImageGenerator {
    async fn generate(&self, prompt: String) -> Result<FunctionResponse, FunctionError> {
        if prompt.is_empty() {
            return make_error_value("prompt is empty");
        }

        info!("generating image with {prompt:?}");
        let request = CreateImageRequest {
            prompt: prompt.clone(),
            model: Some(ImageModel::Other(self.model.clone())),
            ..Default::default()
        };
        let response = match self.client.images().create(request).await {
            Ok(r) => r,
            Err(e) => return make_error_value(&e.to_string()),
        };
        let Some(first_image) = response.data.first() else {
            return make_error_value("no image was generated");
        };
        let Image::Url { url, revised_prompt } = first_image.as_ref() else {
            return make_error_value("invalid response generated");
        };

        let image_url = Url::parse(url).map_err(FunctionError::by_external)?;
        let revised_prompt = revised_prompt.as_ref().unwrap_or(&prompt).to_string();
        let function_response = GenerationResponse {
            image_url: image_url.clone(),
            revised_prompt: revised_prompt.clone(),
        };
        let attachment = ConversationAttachment::Image {
            url: image_url,
            description: Some(revised_prompt),
        };
        Ok(FunctionResponse {
            result: serde_json::to_value(function_response).map_err(FunctionError::by_serialization)?,
            attachments: vec![attachment],
        })
    }
}

fn make_error_value(message: &str) -> Result<FunctionResponse, FunctionError> {
    Ok(FunctionResponse {
        result: serde_json::to_value(GenerationError {
            error: message.to_string(),
        })
        .map_err(FunctionError::by_serialization)?,
        ..Default::default()
    })
}

#[derive(Debug, Serialize)]
struct GenerationResponse {
    image_url: Url,
    revised_prompt: String,
}

#[derive(Debug, Serialize)]
struct GenerationError {
    error: String,
}
