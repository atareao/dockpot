use std::sync::Arc;

use axum::extract::State;
use axum::routing;
use axum::Router;

use crate::auth::AppState;

pub mod agents;
pub mod auth_routes;
pub mod backup;
pub mod convert;
pub mod docker_info;
pub mod env;
pub mod export;
pub mod log_history;
pub mod logs;
pub mod notifiers;
pub mod stacks;
pub mod stats;
pub mod sync;
pub mod templates;

pub fn api_routes() -> Router<Arc<AppState>> {
    let public = Router::new()
        .route("/health", routing::get(health))
        .route("/auth/login", routing::get(auth_routes::login))
        .route("/auth/callback", routing::get(auth_routes::callback));

    let protected = Router::new()
        .route("/api/me", routing::get(auth_routes::me))
        // Stacks
        .route(
            "/api/stacks",
            routing::get(stacks::list).post(stacks::create),
        )
        .route(
            "/api/stacks/{id}",
            routing::get(stacks::get)
                .put(stacks::update)
                .delete(stacks::delete),
        )
        .route("/api/stacks/{id}/start", routing::post(stacks::start))
        .route("/api/stacks/{id}/stop", routing::post(stacks::stop))
        .route("/api/stacks/{id}/restart", routing::post(stacks::restart))
        .route(
            "/api/stacks/{id}/compose",
            routing::get(stacks::get_compose).put(stacks::update_compose),
        )
        .route(
            "/api/stacks/validate",
            routing::post(stacks::validate_compose),
        )
        .route("/api/stacks/{id}/pull", routing::post(stacks::pull))
        .route(
            "/api/stacks/{id}/logs/ws",
            routing::get(logs::logs_ws_handler),
        )
        .route("/api/stacks/{id}/logs", routing::get(log_history::get_logs))
        .route("/api/stacks/{id}/stats", routing::get(stats::stats))
        .route("/api/stacks/{id}/export", routing::get(export::export_zip))
        .route("/api/status", routing::get(stacks::dashboard_status))
        .route("/api/docker/info", routing::get(docker_info::docker_info))
        // Env files
        .route(
            "/api/stacks/{id}/env",
            routing::get(env::list).post(env::upsert),
        )
        .route(
            "/api/stacks/{id}/env/{filename}",
            routing::delete(env::delete),
        )
        // Notifiers
        .route(
            "/api/notifiers",
            routing::get(notifiers::list).post(notifiers::create),
        )
        .route(
            "/api/notifiers/{id}",
            routing::put(notifiers::update).delete(notifiers::delete),
        )
        .route("/api/notifiers/{id}/test", routing::post(notifiers::test))
        .route(
            "/api/stacks/{id}/notifiers",
            routing::get(notifiers::get_stack_notifiers).post(notifiers::set_stack_notifiers),
        )
        // Agents
        .route(
            "/api/agents",
            routing::get(agents::list).post(agents::create),
        )
        .route(
            "/api/agents/{id}",
            routing::get(agents::get)
                .put(agents::update)
                .delete(agents::delete),
        )
        // Convert
        .route(
            "/api/convert/docker-run",
            routing::post(convert::convert_docker_run),
        )
        // Templates
        .route("/api/templates", routing::get(templates::list_templates))
        .route(
            "/api/templates/{name}",
            routing::get(templates::get_template),
        )
        .route(
            "/api/templates/render",
            routing::post(templates::render_template),
        )
        // Backup
        .route(
            "/api/backup/config",
            routing::get(backup::get_config).post(backup::upsert_config),
        )
        .route("/api/backup/run", routing::post(backup::run_now));

    let sync_routes = Router::new()
        .route(
            "/api/stacks/{id}/sync",
            routing::get(sync::get_config).put(sync::set_config),
        )
        .route("/api/stacks/{id}/sync/pull", routing::post(sync::pull))
        .route("/api/stacks/{id}/sync/push", routing::post(sync::push))
        .route("/api/stacks/{id}/sync/diff", routing::get(sync::diff))
        .route("/api/stacks/{id}/sync/status", routing::get(sync::status));

    public.merge(protected).merge(sync_routes)
}

pub async fn health(State(state): State<Arc<AppState>>) -> axum::Json<serde_json::Value> {
    let docker_ok = tokio::process::Command::new("docker")
        .args(["info"])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);
    let db_ok = state.db.get_setting("_health").await.is_ok();

    let status = if docker_ok && db_ok {
        "ok"
    } else if docker_ok {
        "degraded"
    } else {
        "error"
    };

    axum::Json(serde_json::json!({
        "status": status,
        "docker": docker_ok,
        "database": db_ok,
    }))
}
