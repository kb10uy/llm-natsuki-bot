mod auxiliary;
mod conversations;

use crate::application::Application;

use axum::{Router, routing::get};

pub fn routes() -> Router<Application> {
    Router::new().route("/health", get(auxiliary::health))
}
