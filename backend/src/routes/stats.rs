use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::Value;

use crate::state::AppState;

pub async fn stats(
    State(state): State<AppState>,
    Path(stack_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let stats = state
        .db
        .get_stats(&stack_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!(stats)))
}
