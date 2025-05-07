use crate::function::ConfigurableFunction;

use async_openai::types::{Image, ImagesResponse};
use base64::prelude::*;
use futures::{FutureExt, TryFutureExt, future::BoxFuture};
use lnb_common::{config::tools::ConfigToolsImageGenerator, extension::ContextExt};
use lnb_core::{
    APP_USER_AGENT,
    error::FunctionError,
    interface::{
        Context,
        function::{Function, FunctionDescriptor, FunctionResponse},
    },
    model::{
        conversation::{ConversationAttachment, IncompleteConversation},
        message::MessageToolCalling,
        schema::DescribedSchema,
    },
};
use lnb_rate_limiter::{RateLimiter, Rated};
use reqwest::{Client as ReqwestClient, ClientBuilder, Response, header::HeaderMap, multipart::Form};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use tempfile::NamedTempFile;
use thiserror::Error as ThisError;
use time::UtcDateTime;
use tokio::{fs::File, io::AsyncWriteExt};
use tracing::{debug, info};
use url::Url;

pub const LOW_MODERATION_SCOPE: &str = "image_generator:low_moderation";

#[derive(Debug)]
pub struct ImageGenerator {
    http_client: ReqwestClient,
    generate_endpoint: Url,
    edit_endpoint: Url,
    model: String,
    rate_limiter: Option<RateLimiter>,
}

impl ConfigurableFunction for ImageGenerator {
    const NAME: &'static str = stringify!(ImageGenerator);

    type Configuration = ConfigToolsImageGenerator;

    async fn configure(
        config: &ConfigToolsImageGenerator,
        rate_limiter: Option<RateLimiter>,
    ) -> Result<ImageGenerator, FunctionError> {
        let http_client = {
            let mut headers = HeaderMap::new();
            headers.insert(
                "Authorization",
                format!("Bearer {}", config.token).parse().expect("should pass header"),
            );
            ClientBuilder::new()
                .user_agent(APP_USER_AGENT)
                .default_headers(headers)
                .build()
                .map_err(FunctionError::by_external)?
        };
        let generate_endpoint =
            Url::parse(&format!("{}/images/generations", config.endpoint)).map_err(FunctionError::by_serialization)?;
        let edit_endpoint =
            Url::parse(&format!("{}/images/edits", config.endpoint)).map_err(FunctionError::by_serialization)?;

        Ok(ImageGenerator {
            http_client,
            generate_endpoint,
            edit_endpoint,
            model: config.model.to_string(),
            rate_limiter,
        })
    }
}

impl Function for ImageGenerator {
    fn get_descriptor(&self) -> FunctionDescriptor {
        FunctionDescriptor {
            name: "image_generator".to_string(),
            description: r#"
                ユーザーからの要望に基づき、プロンプトの入力から AI を利用して画像を生成・または編集します。
                生成された画像は返答のメッセージに直接添付されます。
            "#
            .to_string(),
            parameters: DescribedSchema::object(
                "parameters",
                "引数",
                vec![
                    DescribedSchema::string_enum(
                        "mode",
                        "動作モードの指定。新しい画像の生成は generate を、既存画像からの編集は edit を指定する。",
                        ["generate", "edit"],
                    ),
                    DescribedSchema::string(
                        "prompt",
                        "GPT-Image, DALL-E などの画像生成モデルに入力するプロンプト文。",
                    ),
                    DescribedSchema::array(
                        "input_image_urls",
                        "edit mode の場合にユーザーから提供される画像の URL のリスト。 generate mode の場合は空にする。",
                        DescribedSchema::string("url", "提供された画像の URL。"),
                    ),
                ],
            ),
        }
    }

    fn call<'a>(
        &'a self,
        context: &'a Context,
        _incomplete: &'a IncompleteConversation,
        tool_calling: MessageToolCalling,
    ) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>> {
        let parameters = match serde_json::from_value(tool_calling.arguments).map_err(FunctionError::by_serialization) {
            Ok(p) => p,
            Err(err) => return async { Err(FunctionError::Serialization(err.into())) }.boxed(),
        };
        async move {
            match self.execute(context, parameters).await {
                Ok(response) => Ok(response),
                Err(IntermediateError::AsResponse(message)) => Ok(FunctionResponse {
                    result: serde_json::to_value(GenerationError {
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

impl ImageGenerator {
    async fn execute(
        &self,
        context: &Context,
        parameters: GenerationParameters,
    ) -> Result<FunctionResponse, IntermediateError> {
        if !self.ensure_in_rate(context.identity()).await {
            return Err(IntermediateError::response("rate limit exceeded"));
        }

        if parameters.prompt.is_empty() {
            return Err(IntermediateError::response("prompt is empty"));
        }

        let images_response = match parameters.mode {
            GenerationMode::Generate => self.generate_image(context, parameters.prompt.clone()).await?,
            GenerationMode::Edit => {
                self.edit_image(context, parameters.prompt.clone(), parameters.input_image_urls)
                    .await?
            }
        };

        let Some(first_image) = images_response.data.first() else {
            return Err(IntermediateError::response("no image was generated"));
        };
        let (image_bytes, returning_prompt) = match first_image.as_ref() {
            Image::Url { url, revised_prompt } => {
                let image_response = self
                    .http_client
                    .get(url)
                    .send()
                    .map_err(FunctionError::by_external)
                    .await?;
                let image_bytes = image_response.bytes().map_err(FunctionError::by_external).await?;
                let attached_prompt = revised_prompt.as_deref().unwrap_or(&parameters.prompt).to_string();
                (image_bytes.into(), attached_prompt)
            }
            Image::B64Json {
                b64_json,
                revised_prompt,
            } => {
                let image_bytes = BASE64_STANDARD
                    .decode(b64_json.as_str())
                    .map_err(FunctionError::by_serialization)?;
                let attached_prompt = revised_prompt.as_deref().unwrap_or(&parameters.prompt).to_string();
                (image_bytes, attached_prompt)
            }
        };

        let function_response = GenerationResponse {
            status: GenerationStatus::GenerationCompleted,
            revised_prompt: returning_prompt.clone(),
        };
        let image_attachment = ConversationAttachment::Image {
            bytes: image_bytes,
            description: Some(returning_prompt),
        };
        Ok(FunctionResponse {
            result: serde_json::to_value(function_response).map_err(FunctionError::by_serialization)?,
            attachments: vec![image_attachment],
        })
    }

    async fn generate_image(&self, context: &Context, prompt: String) -> Result<ImagesResponse, IntermediateError> {
        info!("generating image with {prompt:?}");

        let moderation = if context.role().accepts(LOW_MODERATION_SCOPE) {
            "low"
        } else {
            "auto"
        };
        let request = json!({
            "model": self.model,
            "prompt": prompt,
            "moderation": moderation,
            "user": context.hashed_identity(),
        });

        let raw_response = self
            .http_client
            .post(self.generate_endpoint.clone())
            .json(&request)
            .send()
            .map_err(FunctionError::by_external)
            .await?;

        deserialize_openai_response(raw_response).await
    }

    async fn edit_image(
        &self,
        context: &Context,
        prompt: String,
        input_image_urls: Vec<String>,
    ) -> Result<ImagesResponse, IntermediateError> {
        info!("editing image with {prompt:?}, {} images", input_image_urls.len());

        let mut downloaded_images = vec![];
        for image_url in input_image_urls {
            let image_tempfile = self.download_temporary_image(image_url).await?;
            downloaded_images.push(image_tempfile);
        }

        let form = {
            let mut f = Form::new()
                .text("model", self.model.clone())
                .text("prompt", prompt)
                .text("user", context.identity().unwrap_or("system").to_string());
            for image in &downloaded_images {
                f = f
                    .file("image[]", image.path())
                    .map_err(FunctionError::by_external)
                    .await?;
            }
            f
        };
        let raw_response = self
            .http_client
            .post(self.edit_endpoint.clone())
            .multipart(form)
            .send()
            .map_err(FunctionError::by_external)
            .await?;

        for image in downloaded_images {
            drop(image);
        }

        deserialize_openai_response(raw_response).await
    }

    async fn download_temporary_image(&self, url: String) -> Result<NamedTempFile, IntermediateError> {
        debug!("downloading image from {url}");
        let image_bytes = self
            .http_client
            .get(url)
            .send()
            .map_err(FunctionError::by_external)
            .await?
            .bytes()
            .map_err(FunctionError::by_external)
            .await?;

        let mime_type = infer::get(&image_bytes).map(|ft| ft.mime_type());

        // tempfile に書き出し
        let tempfile = match mime_type {
            Some("image/jpeg") => NamedTempFile::with_suffix(".jpg").map_err(FunctionError::by_external)?,
            Some("image/png") => NamedTempFile::with_suffix(".png").map_err(FunctionError::by_external)?,
            Some("image/gif") => NamedTempFile::with_suffix(".gif").map_err(FunctionError::by_external)?,
            Some("image/webp") => NamedTempFile::with_suffix(".webp").map_err(FunctionError::by_external)?,
            Some(otherwise) => {
                return Err(FunctionError::External(format!("invalid MIME detected: {otherwise}").into()).into());
            }
            None => {
                return Err(FunctionError::External("cannot determine MIME".into()).into());
            }
        };

        debug!("writing temporary image at {:?}", tempfile.path());
        // tokio File にするので分解する
        let (temp_file, temp_path) = tempfile.into_parts();
        let mut async_file = File::from_std(temp_file);
        async_file
            .write_all(&image_bytes)
            .await
            .map_err(FunctionError::by_external)?;
        let restored_file = async_file.into_std().await;
        Ok(NamedTempFile::from_parts(restored_file, temp_path))
    }

    async fn ensure_in_rate(&self, identity: Option<&str>) -> bool {
        let Some(rate_limiter) = &self.rate_limiter else {
            return true;
        };
        let Some(identity) = identity else {
            return false;
        };
        let rated = rate_limiter.check(UtcDateTime::now(), identity).await;
        matches!(rated, Rated::Success)
    }
}

async fn deserialize_openai_response<T: DeserializeOwned>(response: Response) -> Result<T, IntermediateError> {
    let is_success = response.status().is_success();
    let json_value: Value = response.json().map_err(FunctionError::by_serialization).await?;
    if is_success {
        let value = serde_json::from_value(json_value).map_err(FunctionError::by_serialization)?;
        Ok(value)
    } else {
        let error_message = json_value
            .pointer("/error/message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        Err(IntermediateError::AsResponse(error_message.to_string()))
    }
}

#[derive(Debug, Deserialize)]
struct GenerationParameters {
    mode: GenerationMode,
    prompt: String,
    input_image_urls: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GenerationMode {
    Generate,
    Edit,
}

#[derive(Debug, Serialize)]
struct GenerationResponse {
    status: GenerationStatus,
    revised_prompt: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum GenerationStatus {
    GenerationCompleted,
}

#[derive(Debug, Serialize)]
struct GenerationError {
    error: String,
}

#[derive(Debug, ThisError)]
enum IntermediateError {
    #[error("as response error: {0}")]
    AsResponse(String),

    #[error("unrecoverable function error: {0}")]
    Unrecoverable(#[from] FunctionError),
}

impl IntermediateError {
    pub fn response(message: impl Into<String>) -> IntermediateError {
        IntermediateError::AsResponse(message.into())
    }
}
