use std::path::Path as FilePath;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use axum::http::StatusCode;
use serde_json::Value;

use crate::auth::AppState;
use crate::git;
use crate::models::{StackSync, SyncConfigRequest};

pub async fn get_config(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let sync = state
        .db
        .get_sync_config(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get sync config: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    Ok(Json(serde_json::json!(sync)))
}

pub async fn set_config(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<SyncConfigRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let sync_type = req.sync_type.unwrap_or_else(|| "none".to_string());

    let mut sync = StackSync {
        stack_id: id.clone(),
        sync_type,
        remote_url: req.remote_url,
        remote_branch: req.remote_branch.unwrap_or_else(|| "main".to_string()),
        auth_token: req.auth_token,
        last_commit: None,
        last_synced_at: None,
        status: "idle".into(),
    };

    state
        .db
        .upsert_sync_config(&sync)
        .await
        .map_err(|e| {
            tracing::error!("Failed to set sync config: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    // If sync_type changed to git_remote or git_dir, init git repo
    if sync.sync_type != "none" {
        if let Ok(Some(stack)) = state.db.get_stack(&id).await {
            let repo_path = FilePath::new(&stack.path);
            match sync.sync_type.as_str() {
                "git_remote" => {
                    if let Some(url) = &sync.remote_url {
                        match git::sync::clone_remote(url, repo_path, &sync.remote_branch, sync.auth_token.as_deref()) {
                            Ok(repo) => {
                                let commit = git::sync::head_commit(&repo).ok().flatten();
                                state
                                    .db
                                    .update_sync_status(&id, "synced", commit.as_deref())
                                    .await
                                    .ok();
                                sync.status = "synced".into();
                                sync.last_commit = commit;
                            }
                            Err(e) => {
                                tracing::warn!("Failed to clone repo: {}", e);
                            }
                        }
                    }
                }
                "git_dir" => {
                    match git::sync::init_repo(repo_path) {
                        Ok(repo) => {
                            let commit = git::sync::head_commit(&repo).ok().flatten();
                            state
                                .db
                                .update_sync_status(&id, "synced", commit.as_deref())
                                .await
                                .ok();
                            sync.status = "synced".into();
                            sync.last_commit = commit;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to init repo: {}", e);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    tracing::info!("🔁 Sync config updated for stack '{}'", id);
    Ok(Json(serde_json::json!(sync)))
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

    let sync = state
        .db
        .get_sync_config(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get sync config: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Sync not configured".into()))?;

    if sync.sync_type != "git_remote" {
        return Err((StatusCode::BAD_REQUEST, "Sync type is not git_remote".into()));
    }

    let repo_path = FilePath::new(&stack.path);
    let repo = git2::Repository::open(repo_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to open repo: {}", e)))?;

    match git::sync::pull(&repo, &sync.remote_branch) {
        Ok(msg) => {
            let commit = git::sync::head_commit(&repo).ok().flatten();
            state
                .db
                .update_sync_status(&id, "synced", commit.as_deref())
                .await
                .ok();
            tracing::info!("🔽 Git pull for '{}': {}", stack.name, msg);
            Ok(Json(serde_json::json!({"message": msg, "commit": commit})))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Pull failed: {}", e))),
    }
}

pub async fn push(
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

    let sync = state
        .db
        .get_sync_config(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get sync config: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Sync not configured".into()))?;

    if sync.sync_type != "git_remote" {
        return Err((StatusCode::BAD_REQUEST, "Sync type is not git_remote".into()));
    }

    let repo_path = FilePath::new(&stack.path);
    let repo = git2::Repository::open(repo_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to open repo: {}", e)))?;

    // Auto-commit before push
    if git::sync::has_uncommitted(&repo).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))? {
        match git::sync::commit_all(&repo, "dockpot: auto-commit before push") {
            Ok(hash) => tracing::info!("📝 Auto-commit before push: {}", hash),
            Err(e) => tracing::warn!("Auto-commit before push failed: {}", e),
        }
    }

    match git::sync::push(&repo, &sync.remote_branch) {
        Ok(_) => {
            let commit = git::sync::head_commit(&repo).ok().flatten();
            state
                .db
                .update_sync_status(&id, "synced", commit.as_deref())
                .await
                .ok();
            tracing::info!("🔼 Git push for '{}'", stack.name);
            Ok(Json(serde_json::json!({"message": "Push successful", "commit": commit})))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Push failed: {}", e))),
    }
}

pub async fn diff(
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

    let repo_path = FilePath::new(&stack.path);
    let repo = git2::Repository::open(repo_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to open repo: {}", e)))?;

    match git::sync::get_diff(&repo) {
        Ok(diff_data) => Ok(Json(serde_json::json!(diff_data))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Diff failed: {}", e))),
    }
}

pub async fn status(
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

    let sync = state
        .db
        .get_sync_config(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get sync config: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    let sync_status = if let Some(ref s) = sync {
        if s.sync_type == "git_remote" {
            if let Ok(repo) = git2::Repository::open(FilePath::new(&stack.path)) {
                match git::sync::sync_status(&repo, &s.remote_branch) {
                    Ok(s) => s,
                    Err(_) => "unknown".into(),
                }
            } else {
                s.status.clone()
            }
        } else if s.sync_type == "git_dir" {
            if let Ok(repo) = git2::Repository::open(FilePath::new(&stack.path)) {
                match git::sync::has_uncommitted(&repo) {
                    Ok(true) => "pending".into(),
                    Ok(false) => "synced".into(),
                    Err(_) => s.status.clone(),
                }
            } else {
                s.status.clone()
            }
        } else {
            "idle".into()
        }
    } else {
        "none".into()
    };

    Ok(Json(serde_json::json!({
        "stack_id": id,
        "status": sync_status,
    })))
}