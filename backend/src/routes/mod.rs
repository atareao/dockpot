use std::sync::Arc;

use axum::routing;
use axum::Router;

use crate::auth::AppState;

pub mod auth_routes;
pub mod logs;
pub mod stacks;

pub fn api_routes() -> Router<Arc<AppState>> {
    let public = Router::new()
        .route("/health", routing::get(health))
        .route("/auth/login", routing::get(auth_routes::login))
        .route("/auth/callback", routing::get(auth_routes::callback));

    let protected = Router::new()
        .route("/api/me", routing::get(auth_routes::me))
        .route("/api/stacks", routing::get(stacks::list).post(stacks::create))
        .route(
            "/api/stacks/{id}",
            routing::get(stacks::get)
                .put(stacks::update)
                .delete(stacks::delete),
        )
        .route("/api/stacks/{id}/start", routing::post(stacks::start))
        .route("/api/stacks/{id}/stop", routing::post(stacks::stop))
        .route("/api/stacks/{id}/restart", routing::post(stacks::restart))
        .route("/api/stacks/{id}/compose", routing::get(stacks::get_compose).put(stacks::update_compose))
        .route("/api/stacks/validate", routing::post(stacks::validate_compose))
        .route("/api/stacks/{id}/pull", routing::post(stacks::pull))
        .route("/api/stacks/{id}/logs/ws", routing::get(logs::logs_ws_handler))
        .route("/api/status", routing::get(stacks::dashboard_status));

    public.merge(protected)
}

pub async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({"status": "ok"}))
}