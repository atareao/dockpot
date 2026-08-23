use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use axum::http::StatusCode;
use serde_json::Value;
use uuid::Uuid;

use crate::auth::AppState;
use crate::models::BackupSchedule;

pub async fn get_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let config = state.db.get_backup_schedule().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!(config)))
}

pub async fn upsert_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let enabled = req.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
    let cron = req.get("cron_expression").and_then(|v| v.as_str()).unwrap_or("0 3 * * *");
    let retention = req.get("retention_days").and_then(|v| v.as_i64()).unwrap_or(30);
    let include_git = req.get("include_git").and_then(|v| v.as_bool()).unwrap_or(true);
    let include_env = req.get("include_env").and_then(|v| v.as_bool()).unwrap_or(true);

    let now = chrono::Utc::now().to_rfc3339();
    let schedule = BackupSchedule {
        id: uuid::Uuid::new_v4().to_string(),
        enabled,
        cron_expression: cron.to_string(),
        retention_days: retention,
        include_git,
        include_env,
        last_run_at: None,
        last_status: None,
        created_at: now.clone(),
        updated_at: now,
    };

    state.db.upsert_backup_schedule(&schedule).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!(schedule)))
}

pub async fn run_now(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    match state.db.run_backup().await {
        Ok(path) => Ok(Json(serde_json::json!({"status": "ok", "path": path}))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}