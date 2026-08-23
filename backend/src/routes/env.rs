use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use axum::http::StatusCode;
use serde_json::Value;

use crate::auth::AppState;

pub async fn list(
    State(state): State<Arc<AppState>>,
    Path(stack_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let envs = state.db.list_env_files(&stack_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!(envs)))
}

pub async fn upsert(
    State(state): State<Arc<AppState>>,
    Path(stack_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let filename = req.get("filename").and_then(|v| v.as_str()).unwrap_or(".env");
    let content = req.get("content").and_then(|v| v.as_str()).unwrap_or("");

    let env = state.db.upsert_env_file(&stack_id, filename, content).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!(env)))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path((stack_id, filename)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    state.db.delete_env_file(&stack_id, &filename).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}