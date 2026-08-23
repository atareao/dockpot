use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::extract::State;
use axum::Json;
use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::Value;

use crate::auth::AppState;

#[derive(Deserialize)]
pub struct LogsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn get_logs(
    State(state): State<Arc<AppState>>,
    Path(stack_id): Path<String>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let limit = query.limit.unwrap_or(100).min(1000);
    let offset = query.offset.unwrap_or(0);

    let logs = state.db.get_logs(&stack_id, limit, offset).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Format as array of {content, level, created_at}
    let result: Vec<serde_json::Value> = logs.into_iter().map(|(content, level, ts)| {
        serde_json::json!({
            "content": content,
            "level": level,
            "created_at": ts,
        })
    }).collect();

    Ok(Json(serde_json::json!(result)))
}