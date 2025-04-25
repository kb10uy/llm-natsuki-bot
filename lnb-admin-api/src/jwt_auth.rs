use crate::config::ConfigAdminApiJwtAuth;

use std::{
    error::Error,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use anyhow::Result;
use axum::{
    body::Body,
    http::{HeaderName, Request},
    response::Response,
};
use futures::{FutureExt, TryFutureExt, future::ready};
use jsonwebtoken::{
    Algorithm, DecodingKey, TokenData, Validation,
    errors::Error as JwtError,
    jwk::{Jwk, JwkSet},
};
use lnb_core::APP_USER_AGENT;
use reqwest::{Client, ClientBuilder, StatusCode};
use serde::Deserialize;
use thiserror::Error as ThisError;
use time::{Duration, UtcDateTime};
use tokio::sync::RwLock;
use tower::{Layer, Service};
use url::Url;

const JWK_CACHE_EXPIRATION: Duration = Duration::days(1);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct JwtClaims {
    sub: String,
    email: String,
    exp: usize,
}

#[derive(Debug, Clone)]
pub struct JwtAuthLayer {
    jwt_header: HeaderName,
    jwks_url: Url,
    allowed_emails: Vec<String>,

    jwks: Arc<RwLock<Option<JwkSet>>>,
    jwks_fetched_at: Arc<RwLock<UtcDateTime>>,
    client: Client,
}

impl JwtAuthLayer {
    pub fn new(config: ConfigAdminApiJwtAuth) -> Result<JwtAuthLayer> {
        let jwt_header = config.jwt_header_name.parse()?;
        let client = ClientBuilder::new().user_agent(APP_USER_AGENT).build()?;
        Ok(JwtAuthLayer {
            jwt_header,
            jwks_url: config.jwks_url,
            allowed_emails: config.allowed_emails,

            jwks: Arc::new(RwLock::new(None)),
            jwks_fetched_at: Arc::new(RwLock::new(UtcDateTime::UNIX_EPOCH)),
            client,
        })
    }

    fn get_jwt_header<'a, B>(&'a self, req: &'a Request<B>) -> Result<&'a str, JwtAuthError> {
        let jwt_value = req
            .headers()
            .get(&self.jwt_header)
            .and_then(|v| v.to_str().ok())
            .ok_or(JwtAuthError::JwtRequired)?;
        Ok(jwt_value)
    }

    async fn verify(&self, jwt_token: &str) -> Result<JwtClaims, JwtAuthError> {
        let decoding_key = {
            let jwt_header = jsonwebtoken::decode_header(jwt_token)?;
            let key_id = jwt_header.kid.ok_or(JwtAuthError::InvalidJwk)?;
            let Some(jwk) = self.get_jwk(&key_id).await? else {
                return Err(JwtAuthError::JwkNotFound);
            };
            DecodingKey::from_jwk(&jwk)?
        };

        // TODO: issuer とか audience を検証する
        let validation = Validation::new(Algorithm::RS256);

        let token_data: TokenData<JwtClaims> = jsonwebtoken::decode(jwt_token, &decoding_key, &validation)?;

        if !self.allowed_emails.contains(&token_data.claims.email) {
            return Err(JwtAuthError::JwtForbidden);
        }
        Ok(token_data.claims)
    }

    async fn get_jwk(&self, key_id: &str) -> Result<Option<Jwk>, JwtAuthError> {
        let jwks_age = UtcDateTime::now() - *self.jwks_fetched_at.read().await;
        if jwks_age > JWK_CACHE_EXPIRATION {
            self.update_jwks().await?;
        }

        let locked = self.jwks.read().await;
        let locked_jwks = locked.as_ref().expect("should be fetched");
        let jwk = locked_jwks.find(key_id).cloned();
        Ok(jwk)
    }

    async fn update_jwks(&self) -> Result<(), JwtAuthError> {
        let resp = self
            .client
            .get(self.jwks_url.clone())
            .send()
            .map_err(|e| JwtAuthError::JwkFailure(e.into()))
            .await?;
        let jwks: JwkSet = resp.json().map_err(|e| JwtAuthError::JwkFailure(e.into())).await?;

        let mut locked_jwks = self.jwks.write().await;
        let mut locked_jwks_fetched_at = self.jwks_fetched_at.write().await;
        *locked_jwks = Some(jwks);
        *locked_jwks_fetched_at = UtcDateTime::now();

        Ok(())
    }
}

impl<S> Layer<S> for JwtAuthLayer {
    type Service = JwtAuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        JwtAuthMiddleware {
            layer: self.clone(),
            inner,
        }
    }
}

#[derive(Clone)]
pub struct JwtAuthMiddleware<S> {
    layer: JwtAuthLayer,
    inner: S,
}

impl<S, B> Service<Request<B>> for JwtAuthMiddleware<S>
where
    S: Service<Request<B>, Response = Response> + Send + Clone + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<S::Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let layer = self.layer.clone();
        let mut inner = self.inner.clone();

        async move {
            match ready(layer.get_jwt_header(&req))
                .and_then(|jwt| layer.verify(jwt))
                .await
            {
                Ok(c) => {
                    let mut request = req;
                    request.extensions_mut().insert(c);
                    inner.call(request).await
                }
                Err(err) => Ok(unauthorized_response(err)),
            }
        }
        .boxed()
    }
}

fn unauthorized_response(err: JwtAuthError) -> Response<Body> {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(format!("Authentication Error: {err}").into())
        .expect("should be consructed")
}

#[derive(Debug, ThisError)]
pub enum JwtAuthError {
    #[error("JWT header required")]
    JwtRequired,

    #[error("forbidden")]
    JwtForbidden,

    #[error("invalid JWT")]
    InvalidJwk,

    #[error("corresponding JWK not found")]
    JwkNotFound,

    #[error("JWT failure: {0}")]
    JwtError(#[from] JwtError),

    #[error("JWK failure: {0}")]
    JwkFailure(Box<dyn Send + Sync + Error + 'static>),
}
