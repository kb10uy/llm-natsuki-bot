mod auxiliary;
mod conversations;
mod error;
mod reminders;

use crate::{application::Application, jwt_auth::JwtAuthLayer};

use axum::{Router, routing::get};
use lnb_common::config::admin_api::ConfigAdminApi;
use tower_http::cors::CorsLayer;
use tracing::info;

pub fn routes(config_admin_api: &ConfigAdminApi) -> Router<Application> {
    let mut api_routes = Router::new()
        .route("/health", get(auxiliary::health))
        .route("/conversations/count", get(conversations::count))
        .route("/conversations/show", get(conversations::show))
        .route("/conversations/latest_ids", get(conversations::latest_ids))
        .route("/reminders/count", get(reminders::count));

    // JWT Auth
    if let Some(auth_config) = &config_admin_api.jwt_auth {
        api_routes = api_routes.layer(JwtAuthLayer::new(auth_config.clone()));
        info!("JWT authentication enabled");
    }
    // CORS
    if let Some(cors_config) = &config_admin_api.cors {
        let header_origin: Vec<_> = cors_config
            .allowed_origins
            .iter()
            .map(|o| o.parse().expect("invalid origin"))
            .collect();
        let cors_layer = CorsLayer::new().allow_origin(header_origin).allow_credentials(true);
        api_routes = api_routes.layer(cors_layer);
        info!("CORS setting applied");
    }

    Router::new().nest("/api", api_routes)
}
