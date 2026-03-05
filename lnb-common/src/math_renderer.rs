use lnb_core::APP_USER_AGENT;
use reqwest::{Client, Error as ReqwestError};
use serde::Deserialize;
use serde_json::json;
use thiserror::Error as ThisError;

#[derive(Debug, Clone)]
pub struct MathRendererClient {
    client: Client,
    base_url: String,
    scale: f64,
}

#[derive(Debug, ThisError)]
pub enum MathRendererError {
    #[error("math renderer communication error: {0}")]
    Communication(#[from] ReqwestError),

    #[error("rendering error: {0}")]
    Rendering(String),
}

#[derive(Debug, Clone, Deserialize)]
struct RenderingError {
    error: String,
}

impl MathRendererClient {
    pub fn new(base_url: impl Into<String>, scale: f64) -> Result<MathRendererClient, MathRendererError> {
        let client = reqwest::ClientBuilder::new().user_agent(APP_USER_AGENT).build()?;
        Ok(MathRendererClient {
            client,
            base_url: base_url.into(),
            scale,
        })
    }

    pub async fn render(&self, formula: &str, display_mode: bool) -> Result<Vec<u8>, MathRendererError> {
        let payload = json!({
            "formula": formula,
            "display": display_mode,
            "scale": self.scale,
        });
        let resp = self
            .client
            .post(format!("{}/renderMath", self.base_url))
            .json(&payload)
            .send()
            .await?;
        let png_bytes = if resp.status().is_success() {
            resp.bytes().await?
        } else {
            let error: RenderingError = resp.json().await?;
            return Err(MathRendererError::Rendering(error.error));
        };

        Ok(png_bytes.to_vec())
    }

    pub async fn render_multiple(
        &self,
        formulae: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Vec<u8>, MathRendererError> {
        let formulae: Vec<_> = formulae.into_iter().map(|f| f.into()).collect();
        let payload = json!({
            "formulae": formulae,
            "scale": self.scale,
        });
        let resp = self
            .client
            .post(format!("{}/renderMathMultiple", self.base_url))
            .json(&payload)
            .send()
            .await?;
        let png_bytes = if resp.status().is_success() {
            resp.bytes().await?
        } else {
            let error: RenderingError = resp.json().await?;
            return Err(MathRendererError::Rendering(error.error));
        };

        Ok(png_bytes.to_vec())
    }
}
