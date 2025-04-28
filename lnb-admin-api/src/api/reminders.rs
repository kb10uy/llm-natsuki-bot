use crate::{api::error::ApiError, application::Application};

use axum::{Json, extract::State};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CountResponse {
    count: usize,
}
pub async fn count(State(state): State<Application>) -> Result<Json<CountResponse>, ApiError> {
    let count = state.reminder.count().await?;
    Ok(Json(CountResponse { count }))
}
