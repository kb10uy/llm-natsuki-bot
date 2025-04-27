use axum::{Router, routing::get};

mod auxiliary;
mod conversations;

pub fn routes() -> Router<()> {
    Router::new().route("/health", get(auxiliary::health))
}
