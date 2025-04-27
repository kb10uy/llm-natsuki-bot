use crate::application::ApplicationError;

use axum::{
    Json,
    response::{IntoResponse, Response},
};
use reqwest::StatusCode;
use serde::Serialize;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum ApiError {
    #[error("application error: {0}")]
    Application(#[from] ApplicationError),

    #[error("invalid request: {0}")]
    InvalidRequest(String),
}

#[derive(Debug, Clone, Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            ApiError::Application(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            ApiError::InvalidRequest(message) => (StatusCode::UNPROCESSABLE_ENTITY, message),
        };
        (status, Json(ErrorResponse { error })).into_response()
    }
}
