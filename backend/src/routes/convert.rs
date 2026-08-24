use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde_json::Value;

use crate::auth::AppState;
use crate::convert;
use crate::models::DockerRunRequest;

pub async fn convert_docker_run(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<DockerRunRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let service_name = req.service_name.unwrap_or_else(|| "app".to_string());

    match convert::docker_run_to_compose(&req.command, &service_name) {
        Ok(compose) => Ok(Json(serde_json::json!({
            "service_name": service_name,
            "compose": compose,
            "valid": true,
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "valid": false,
            "error": e,
        }))),
    }
}
