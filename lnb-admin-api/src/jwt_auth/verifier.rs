use crate::jwt_auth::{JwtAuthError, JwtClaims};

use std::sync::Arc;

use futures::TryFutureExt;
use jsonwebtoken::{
    Algorithm, DecodingKey, TokenData, Validation,
    jwk::{Jwk, JwkSet},
};
use lnb_core::APP_USER_AGENT;
use reqwest::{Client, ClientBuilder};
use time::{Duration, UtcDateTime};
use tokio::sync::RwLock;
use tracing::info;
use url::Url;

const JWK_CACHE_EXPIRATION: Duration = Duration::days(1);

#[derive(Debug, Clone)]
pub struct JwtVerifier {
    validation: Validation,
    allowed_subjects: Vec<String>,
    jwk_cache: JwkCache,
}

impl JwtVerifier {
    pub fn new(jwks_url: Url, audience: String, allowed_subjects: Vec<String>) -> JwtVerifier {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[audience]);

        JwtVerifier {
            validation,
            allowed_subjects,
            jwk_cache: JwkCache::new(jwks_url),
        }
    }

    pub async fn verify(&self, jwt_token: &str) -> Result<JwtClaims, JwtAuthError> {
        let decoding_key = {
            let jwt_header = jsonwebtoken::decode_header(jwt_token)?;
            let key_id = jwt_header.kid.ok_or(JwtAuthError::InvalidJwk)?;
            let Some(jwk) = self.jwk_cache.get(&key_id).await? else {
                return Err(JwtAuthError::JwkNotFound);
            };
            DecodingKey::from_jwk(&jwk)?
        };

        let token_data: TokenData<JwtClaims> = jsonwebtoken::decode(jwt_token, &decoding_key, &self.validation)?;
        let claims = token_data.claims;

        if self.allowed_subjects.contains(&claims.sub) {
            Ok(claims)
        } else {
            Err(JwtAuthError::SubjectNotAllowed(claims.sub))
        }
    }
}

#[derive(Debug, Clone)]
struct JwkCache {
    client: Client,
    url: Url,
    jwks: Arc<RwLock<Option<JwkSet>>>,
    jwks_fetched_at: Arc<RwLock<UtcDateTime>>,
}

impl JwkCache {
    pub fn new(url: Url) -> JwkCache {
        let client = ClientBuilder::new()
            .user_agent(APP_USER_AGENT)
            .build()
            .expect("client should build");
        JwkCache {
            client,
            url,
            jwks: Arc::new(RwLock::new(None)),
            jwks_fetched_at: Arc::new(RwLock::new(UtcDateTime::UNIX_EPOCH)),
        }
    }

    pub async fn get(&self, key_id: &str) -> Result<Option<Jwk>, JwtAuthError> {
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
            .get(self.url.clone())
            .send()
            .map_err(|e| JwtAuthError::JwkFailure(e.into()))
            .await?;
        let jwks: JwkSet = resp.json().map_err(|e| JwtAuthError::JwkFailure(e.into())).await?;

        info!("JWKs cache updated; {} keys registered", jwks.keys.len());
        let mut locked_jwks = self.jwks.write().await;
        let mut locked_jwks_fetched_at = self.jwks_fetched_at.write().await;
        *locked_jwks = Some(jwks);
        *locked_jwks_fetched_at = UtcDateTime::now();
        Ok(())
    }
}
