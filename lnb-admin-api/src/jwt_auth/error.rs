use std::error::Error as StdError;

use jsonwebtoken::errors::Error as JwtError;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum JwtAuthError {
    #[error("JWT header required")]
    JwtRequired,

    #[error("invalid JWT")]
    InvalidJwk,

    #[error("corresponding JWK not found")]
    JwkNotFound,

    #[error("JWT failure: {0}")]
    JwtError(#[from] JwtError),

    #[error("JWK failure: {0}")]
    JwkFailure(Box<dyn Send + Sync + StdError + 'static>),

    #[error("subject '{0}' is not allowed")]
    SubjectNotAllowed(String),
}
