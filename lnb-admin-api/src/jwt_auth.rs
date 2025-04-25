mod error;
mod layer;
mod verifier;

pub use error::JwtAuthError;
pub use layer::JwtAuthLayer;
pub use verifier::JwtVerifier;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct JwtClaims {
    sub: String,
    email: String,
    exp: usize,
}
