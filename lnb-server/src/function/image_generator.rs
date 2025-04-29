use crate::ConfigurableSimpleFunction;

use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{CreateImageRequest, Image, ImageModel, ImagesResponse},
};
use base64::prelude::*;
use futures::{FutureExt, TryFutureExt, future::BoxFuture};
use lnb_common::config::tools::ConfigToolsImageGenerator;
use lnb_core::{
    APP_USER_AGENT,
    error::FunctionError,
    interface::function::{FunctionDescriptor, FunctionResponse, simple::SimpleFunction},
    model::{conversation::ConversationAttachment, schema::DescribedSchema},
};
use reqwest::{Client as ReqwestClient, ClientBuilder, header::HeaderMap, multipart::Form};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tempfile::NamedTempFile;
use thiserror::Error as ThisError;
use tokio::{fs::File, io::AsyncWriteExt};
use tracing::{debug, info};
use url::Url;

#[derive(Debug)]
pub struct ImageGenerator {
    client: Client<OpenAIConfig>,
    http_client: ReqwestClient,
    edit_endpoint: Url,
    model: String,
}

impl ConfigurableSimpleFunction for ImageGenerator {
    const NAME: &'static str = stringify!(ImageGenerator);

    type Configuration = ConfigToolsImageGenerator;

    async fn configure(config: &ConfigToolsImageGenerator) -> Result<ImageGenerator, FunctionError> {
        let client = {
            let openai_config = OpenAIConfig::new()
                .with_api_key(&config.token)
                .with_api_base(&config.endpoint);
            let http_client = ClientBuilder::new()
                .user_agent(APP_USER_AGENT)
                .build()
                .map_err(|e| FunctionError::External(e.into()))?;
            Client::with_config(openai_config).with_http_client(http_client)
        };
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
        let edit_endpoint =
            Url::parse(&format!("{}/images/edits", config.endpoint)).map_err(FunctionError::by_serialization)?;
        Ok(ImageGenerator {
            client,
            http_client,
            edit_endpoint,
            model: config.model.to_string(),
        })
    }
}

impl SimpleFunction for ImageGenerator {
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

    fn call<'a>(&'a self, _id: &str, params: Value) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>> {
        let parameters = match serde_json::from_value(params).map_err(FunctionError::by_serialization) {
            Ok(p) => p,
            Err(err) => return async { Err(FunctionError::Serialization(err.into())) }.boxed(),
        };
        async move {
            match self.execute(parameters).await {
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
    async fn execute(&self, parameters: GenerationParameters) -> Result<FunctionResponse, IntermediateError> {
        if parameters.prompt.is_empty() {
            return Err(IntermediateError::AsResponse("prompt is empty".to_string()));
        }

        let images_response = match parameters.mode {
            GenerationMode::Generate => self.generate_image(parameters.prompt.clone()).await?,
            GenerationMode::Edit => {
                self.edit_image(parameters.prompt.clone(), parameters.input_image_urls)
                    .await?
            }
        };

        let Some(first_image) = images_response.data.first() else {
            return Err(IntermediateError::AsResponse("no image was generated".to_string()));
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

    async fn generate_image(&self, prompt: String) -> Result<ImagesResponse, IntermediateError> {
        info!("generating image with {prompt:?}");

        let request = CreateImageRequest {
            prompt: prompt.clone(),
            model: Some(ImageModel::Other(self.model.clone())),
            ..Default::default()
        };
        self.client
            .images()
            .create(request)
            .map_err(|e| IntermediateError::AsResponse(e.to_string()))
            .await
    }

    async fn edit_image(
        &self,
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
            let mut f = Form::new().text("model", self.model.clone()).text("prompt", prompt);
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

        let response = raw_response.json().map_err(FunctionError::by_serialization).await?;
        Ok(response)
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
