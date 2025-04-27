use crate::{api::error::ApiError, application::Application};

use axum::{Json, extract::State, response::IntoResponse};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct CountResponse {
    count: usize,
}

pub async fn count(State(state): State<Application>) -> Result<impl IntoResponse, ApiError> {
    let count = state.conversation.count().await?;
    Ok(Json(CountResponse { count }))
}
