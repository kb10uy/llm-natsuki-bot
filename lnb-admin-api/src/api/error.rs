use axum::{
    Json,
    response::{IntoResponse, Response},
};
use lnb_common::persistence::PersistenceError;
use reqwest::StatusCode;
use serde::Serialize;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum ApiError {
    #[error("persistence layer error: {0}")]
    Persistence(#[from] PersistenceError),

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("not found")]
    NotFound,
}

#[derive(Debug, Clone, Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            ApiError::Persistence(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            ApiError::InvalidRequest(message) => (StatusCode::UNPROCESSABLE_ENTITY, message),
            ApiError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
        };
        (status, Json(ErrorResponse { error })).into_response()
    }
}
