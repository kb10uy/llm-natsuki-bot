use axum::{Extension, response::IntoResponse};

use crate::jwt_auth::JwtClaims;

pub async fn health(Extension(claims): Extension<JwtClaims>) -> impl IntoResponse {
    format!("Authenticated. Hello, {} ! (sub: {})", claims.email, claims.sub)
}
