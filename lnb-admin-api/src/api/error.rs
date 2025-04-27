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
}

#[derive(Debug, Clone, Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            ApiError::Application(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
        };
        (status, Json(ErrorResponse { error })).into_response()
    }
}
