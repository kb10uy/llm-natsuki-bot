use crate::{
    api::error::ApiError,
    application::{Application, ApplicationError},
};

use axum::{
    Json,
    extract::{Query, State},
};
use lnb_core::model::conversation::Conversation;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

const FETCH_COUNT_DEFAULT: usize = 20;
const FETCH_COUNT_MAX: usize = 50;

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

#[derive(Debug, Deserialize)]
pub struct LatestIdsRequest {
    count: Option<usize>,
    max_id: Option<Uuid>,
    min_id: Option<Uuid>,
}
#[derive(Debug, Serialize)]
pub struct LatestIdsResponseItem {
    id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}
pub async fn latest_ids(
    State(state): State<Application>,
    request: Query<LatestIdsRequest>,
) -> Result<Json<Vec<LatestIdsResponseItem>>, ApiError> {
    let fetching_count = request.count.unwrap_or(FETCH_COUNT_DEFAULT).min(FETCH_COUNT_MAX);
    let fetched_ids = match (request.min_id, request.max_id) {
        // 降順
        (None, Some(max)) => state.conversation.latest_ids(fetching_count, Some(max)).await?,
        (None, None) => state.conversation.latest_ids(fetching_count, None).await?,

        // 昇順
        (Some(min), None) => {
            let mut ids = state.conversation.earliest_ids(fetching_count, Some(min)).await?;
            ids.reverse();
            ids
        }

        (Some(_), Some(_)) => return Err(ApiError::InvalidRequest("max_id and min_id are exclusive".to_string())),
    };

    let response_items: Result<Vec<_>, ApiError> = fetched_ids
        .into_iter()
        .map(|id| {
            let (timestamp, _) = id
                .get_timestamp()
                .ok_or_else(|| ApplicationError::Serialization("invalid ID".into()))?
                .to_unix();
            let created_at =
                OffsetDateTime::from_unix_timestamp(timestamp as i64).map_err(ApplicationError::by_serialization)?;
            Ok(LatestIdsResponseItem { id, created_at })
        })
        .collect();
    Ok(Json(response_items?))
}
