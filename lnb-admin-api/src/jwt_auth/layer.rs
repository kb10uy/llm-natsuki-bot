use crate::{
    config::ConfigAdminApiJwtAuth,
    jwt_auth::{JwtAuthError, JwtVerifier},
};

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use axum::{
    body::Body,
    http::{HeaderName, Request},
    response::Response,
};
use futures::FutureExt;
use reqwest::StatusCode;
use tower::{Layer, Service};

#[derive(Debug, Clone)]
pub struct JwtAuthLayer {
    jwt_header: HeaderName,
    verifier: JwtVerifier,
}

impl JwtAuthLayer {
    pub fn new(config: ConfigAdminApiJwtAuth) -> JwtAuthLayer {
        let jwt_header = config.jwt_header_name.parse().expect("invalid header name");
        let verifier = JwtVerifier::new(config.jwks_url, config.audience, config.allowed_subjects);
        JwtAuthLayer { jwt_header, verifier }
    }

    fn get_jwt_header<'a, B>(&'a self, req: &'a Request<B>) -> Result<&'a str, JwtAuthError> {
        let jwt_value = req
            .headers()
            .get(&self.jwt_header)
            .and_then(|v| v.to_str().ok())
            .ok_or(JwtAuthError::JwtRequired)?;
        Ok(jwt_value)
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
            let Ok(jwt_token) = layer.get_jwt_header(&req) else {
                return Ok(unauthorized_response(JwtAuthError::JwtRequired));
            };

            match layer.verifier.verify(jwt_token).await {
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
