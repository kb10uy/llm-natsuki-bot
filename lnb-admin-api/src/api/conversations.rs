use crate::{api::error::ApiError, application::Application};

use axum::{
    Json,
    extract::{Query, State},
};
use lnb_core::model::conversation::Conversation;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct CountResponse {
    count: usize,
}
pub async fn count(State(state): State<Application>) -> Result<Json<CountResponse>, ApiError> {
    let count = state.conversation.count().await?;
    Ok(Json(CountResponse { count }))
}

#[derive(Debug, Deserialize)]

pub struct ShowRequest {
    id: Uuid,
}
pub async fn show(
    State(state): State<Application>,
    request: Query<ShowRequest>,
) -> Result<Json<Conversation>, ApiError> {
    let conversation = state.conversation.show(request.id).await?;
    Ok(Json(conversation))
}
