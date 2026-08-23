use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use axum::http::StatusCode;
use serde_json::Value;
use uuid::Uuid;

use crate::auth::AppState;
use crate::models::{CreateNotifierRequest, Notifier};
use crate::notifier;

pub async fn list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let notifiers = state.db.list_notifiers().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;
    Ok(Json(serde_json::json!(notifiers)))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateNotifierRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    if req.name.trim().is_empty() || req.notifier_type.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Name and type are required".into()));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let notifier = Notifier {
        id: Uuid::new_v4().to_string(),
        name: req.name,
        notifier_type: req.notifier_type,
        config_json: req.config_json.unwrap_or_default(),
        enabled: true,
        created_at: now.clone(),
        updated_at: now,
    };

    state.db.create_notifier(&notifier).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    tracing::info!("📢 Notifier '{}' created ({})", notifier.name, notifier.notifier_type);
    Ok(Json(serde_json::json!(notifier)))
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateNotifierRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let existing = state.db.get_notifier(&id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Notifier '{}' not found", id)))?;

    let notifier = Notifier {
        id: id.clone(),
        name: req.name,
        notifier_type: req.notifier_type,
        config_json: req.config_json.unwrap_or(existing.config_json),
        enabled: existing.enabled,
        created_at: existing.created_at,
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    state.db.update_notifier(&id, &notifier).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok(Json(serde_json::json!(notifier)))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let deleted = state.db.delete_notifier(&id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if !deleted {
        return Err((StatusCode::NOT_FOUND, format!("Notifier '{}' not found", id)));
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn test(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let notifier = state.db.get_notifier(&id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Notifier '{}' not found", id)))?;

    notifier::send_notification(
        &notifier.notifier_type,
        &notifier.config_json,
        "🧪 Dockpot Test",
        "This is a test notification from Dockpot",
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(serde_json::json!({"status": "ok", "message": "Test notification sent"})))
}

pub async fn set_stack_notifiers(
    State(state): State<Arc<AppState>>,
    Path(stack_id): Path<String>,
    Json(ids): Json<Vec<String>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    state.db.set_stack_notifiers(&stack_id, &ids).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

pub async fn get_stack_notifiers(
    State(state): State<Arc<AppState>>,
    Path(stack_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let ids = state.db.get_stack_notifier_ids(&stack_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!(ids)))
}