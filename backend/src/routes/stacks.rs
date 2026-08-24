use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::Value;

use crate::auth::AppState;
use crate::git;
use crate::models::{CreateStackRequest, UpdateStackRequest};

/// Auto-commit changes if the stack has git sync enabled
async fn auto_commit(state: &Arc<AppState>, id: &str, message: &str) {
    if let Ok(Some(sync)) = state.db.get_sync_config(id).await {
        if sync.sync_type != "none" {
            if let Ok(Some(stack)) = state.db.get_stack(id).await {
                let repo_path = std::path::Path::new(&stack.path);
                if let Ok(repo) = git2::Repository::open(repo_path) {
                    if let Err(e) = git::sync::commit_all(&repo, message) {
                        tracing::warn!("Auto-commit failed for '{}': {}", stack.name, e);
                    } else {
                        tracing::info!("📝 Auto-commit for '{}': {}", stack.name, message);
                        let commit = git::sync::head_commit(&repo).ok().flatten();
                        state
                            .db
                            .update_sync_status(id, "pending", commit.as_deref())
                            .await
                            .ok();
                    }
                }
            }
        }
    }
}

pub async fn list(State(state): State<Arc<AppState>>) -> Result<Json<Value>, (StatusCode, String)> {
    let stacks = state.db.list_stacks().await.map_err(|e| {
        tracing::error!("Failed to list stacks: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok(Json(serde_json::json!(stacks)))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateStackRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    if req.name.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Name is required".into()));
    }

    // Check name uniqueness
    if let Ok(Some(_)) = state.db.get_stack_by_name(&req.name).await {
        return Err((
            StatusCode::CONFLICT,
            format!("Stack '{}' already exists", req.name),
        ));
    }

    let compose = req.compose.unwrap_or_else(|| r#"version: "3""#.to_string());

    let stack = state
        .db
        .create_stack(&req.name, req.description.as_deref(), &compose)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create stack: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    tracing::info!("✅ Stack '{}' created", stack.name);

    auto_commit(
        &state,
        &stack.id,
        &format!("dockpot: create stack '{}'", stack.name),
    )
    .await;

    Ok(Json(serde_json::json!(stack)))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let stack = state
        .db
        .get_stack(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get stack: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Stack '{}' not found", id)))?;

    Ok(Json(serde_json::json!(stack)))
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateStackRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let existing = state
        .db
        .get_stack(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get stack: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Stack '{}' not found", id)))?;

    let name = req.name.unwrap_or(existing.name);
    let description = req.description.or(existing.description.clone());
    let compose = req.compose.unwrap_or(existing.compose);

    let updated = state
        .db
        .update_stack(&id, &name, description.as_deref(), &compose)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update stack: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    if !updated {
        return Err((StatusCode::NOT_FOUND, format!("Stack '{}' not found", id)));
    }

    let stack = state.db.get_stack(&id).await.unwrap().unwrap();

    auto_commit(
        &state,
        &id,
        &format!("dockpot: update stack '{}'", stack.name),
    )
    .await;

    Ok(Json(serde_json::json!(stack)))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let deleted = state.db.delete_stack(&id).await.map_err(|e| {
        tracing::error!("Failed to delete stack: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    if !deleted {
        return Err((StatusCode::NOT_FOUND, format!("Stack '{}' not found", id)));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn start(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let stack = state
        .db
        .get_stack(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get stack: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Stack '{}' not found", id)))?;

    let compose_path = format!("{}/compose.yaml", stack.path);

    // Run docker compose up -d
    let output = tokio::process::Command::new("docker")
        .args(["compose", "-f", &compose_path, "up", "-d"])
        .output()
        .await
        .map_err(|e| {
            tracing::error!("Docker compose up failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Docker error: {}", e),
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Docker error: {}", stderr),
        ));
    }

    state.db.update_status(&id, "running").await.ok();
    state.db.record_start(&id).await.ok();
    state
        .db
        .append_logs(&id, &format!("✅ Stack '{}' started\n", stack.name), "info")
        .await
        .ok();

    // Send notification
    if let Ok(notifier_ids) = state.db.get_stack_notifier_ids(&id).await {
        for nid in &notifier_ids {
            if let Ok(Some(notifier)) = state.db.get_notifier(nid).await {
                if notifier.enabled {
                    let _ = crate::notifier::send_notification(
                        &notifier.notifier_type,
                        &notifier.config_json,
                        &format!("✅ {} started", stack.name),
                        &format!("Stack '{}' has been deployed successfully.", stack.name),
                    )
                    .await;
                }
            }
        }
    }

    tracing::info!("✅ Stack '{}' started", stack.name);

    let stack = state.db.get_stack(&id).await.unwrap().unwrap();
    Ok(Json(serde_json::json!(stack)))
}

pub async fn stop(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let stack = state
        .db
        .get_stack(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get stack: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Stack '{}' not found", id)))?;

    let compose_path = format!("{}/compose.yaml", stack.path);

    // Run docker compose down
    let output = tokio::process::Command::new("docker")
        .args(["compose", "-f", &compose_path, "down"])
        .output()
        .await
        .map_err(|e| {
            tracing::error!("Docker compose down failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Docker error: {}", e),
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Docker error: {}", stderr),
        ));
    }

    state.db.update_status(&id, "stopped").await.ok();
    state.db.record_stop(&id).await.ok();
    state
        .db
        .append_logs(
            &id,
            &format!("⏹️  Stack '{}' stopped\n", stack.name),
            "info",
        )
        .await
        .ok();

    // Send notification
    if let Ok(notifier_ids) = state.db.get_stack_notifier_ids(&id).await {
        for nid in &notifier_ids {
            if let Ok(Some(notifier)) = state.db.get_notifier(nid).await {
                if notifier.enabled {
                    let _ = crate::notifier::send_notification(
                        &notifier.notifier_type,
                        &notifier.config_json,
                        &format!("⏹️ {} stopped", stack.name),
                        &format!("Stack '{}' has been stopped.", stack.name),
                    )
                    .await;
                }
            }
        }
    }

    tracing::info!("⏹️  Stack '{}' stopped", stack.name);

    let stack = state.db.get_stack(&id).await.unwrap().unwrap();
    Ok(Json(serde_json::json!(stack)))
}

pub async fn restart(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let stack = state
        .db
        .get_stack(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get stack: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Stack '{}' not found", id)))?;

    let compose_path = format!("{}/compose.yaml", stack.path);

    // Run docker compose restart
    let output = tokio::process::Command::new("docker")
        .args(["compose", "-f", &compose_path, "restart"])
        .output()
        .await
        .map_err(|e| {
            tracing::error!("Docker compose restart failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Docker error: {}", e),
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Docker error: {}", stderr),
        ));
    }

    state.db.update_status(&id, "running").await.ok();
    tracing::info!("🔄 Stack '{}' restarted", stack.name);

    let stack = state.db.get_stack(&id).await.unwrap().unwrap();
    Ok(Json(serde_json::json!(stack)))
}

pub async fn get_compose(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let stack = state
        .db
        .get_stack(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get stack: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Stack '{}' not found", id)))?;

    Ok(Json(serde_json::json!({
        "id": stack.id,
        "name": stack.name,
        "compose": stack.compose,
    })))
}

pub async fn update_compose(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let compose = req
        .get("compose")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "compose field is required".into()))?;

    let stack = state
        .db
        .get_stack(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get stack: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Stack '{}' not found", id)))?;

    state
        .db
        .update_stack(&id, &stack.name, stack.description.as_deref(), compose)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update compose: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    tracing::info!("📝 Compose updated for stack '{}'", stack.name);

    auto_commit(
        &state,
        &id,
        &format!("dockpot: update compose '{}'", stack.name),
    )
    .await;

    let stack = state.db.get_stack(&id).await.unwrap().unwrap();
    Ok(Json(serde_json::json!(stack)))
}

pub async fn validate_compose(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let compose = req
        .get("compose")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "compose field is required".into()))?;

    match serde_yaml::from_str::<serde_yaml::Value>(compose) {
        Ok(_) => Ok(Json(serde_json::json!({"valid": true}))),
        Err(e) => Ok(Json(
            serde_json::json!({"valid": false, "error": e.to_string()}),
        )),
    }
}

pub async fn pull(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let stack = state
        .db
        .get_stack(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get stack: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Stack '{}' not found", id)))?;

    let compose_path = format!("{}/compose.yaml", stack.path);

    let output = tokio::process::Command::new("docker")
        .args(["compose", "-f", &compose_path, "pull"])
        .output()
        .await
        .map_err(|e| {
            tracing::error!("Docker compose pull failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Docker error: {}", e),
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Docker error: {}", stderr),
        ));
    }

    tracing::info!("📥 Images pulled for stack '{}'", stack.name);
    Ok(Json(serde_json::json!(stack)))
}

pub async fn dashboard_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let status = state.db.get_dashboard_status().await.map_err(|e| {
        tracing::error!("Failed to get dashboard status: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok(Json(serde_json::json!(status)))
}
