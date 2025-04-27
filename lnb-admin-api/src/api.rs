mod auxiliary;
mod conversations;
mod error;

use crate::application::Application;

use axum::{Router, routing::get};

pub fn routes() -> Router<Application> {
    Router::new()
        .route("/health", get(auxiliary::health))
        .route("/conversations/count", get(conversations::count))
}
