mod auxiliary;
mod conversations;
mod error;
mod reminders;

use crate::application::Application;

use axum::{Router, routing::get};

pub fn routes() -> Router<Application> {
    Router::new()
        .route("/health", get(auxiliary::health))
        .route("/conversations/count", get(conversations::count))
        .route("/conversations/show", get(conversations::show))
        .route("/conversations/latest_ids", get(conversations::latest_ids))
        .route("/reminders/count", get(reminders::count))
}
