use axum::extract::State;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct LogsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn get_logs(
    State(state): State<AppState>,
    Path(stack_id): Path<String>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let limit = query.limit.unwrap_or(100).min(1000);
    let offset = query.offset.unwrap_or(0);

    let logs = state
        .db
        .get_logs(&stack_id, limit, offset)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let result: Vec<serde_json::Value> = logs.into_iter().map(|(content, level, ts)| {
        serde_json::json!({"content": content, "level": level, "created_at": ts})
    }).collect();

    Ok(Json(serde_json::json!(result)))
}
