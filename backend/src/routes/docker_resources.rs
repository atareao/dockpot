use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};

use crate::state::AppState;

/// `GET /api/docker/volumes` — list Docker volumes via Bollard
pub async fn list_volumes(
    State(state): State<AppState>,
) -> Result<Json<Vec<Value>>, (StatusCode, String)> {
    let volumes = state
        .docker
        .list_volumes::<String>(None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items: Vec<Value> = volumes
        .volumes
        .unwrap_or_default()
        .into_iter()
        .map(|v| {
            json!({
                "name": v.name,
                "driver": v.driver,
                "mountpoint": v.mountpoint,
            })
        })
        .collect();

    Ok(Json(items))
}

/// `GET /api/docker/networks` — list Docker networks via Bollard
pub async fn list_networks(
    State(state): State<AppState>,
) -> Result<Json<Vec<Value>>, (StatusCode, String)> {
    let networks = state
        .docker
        .list_networks::<String>(None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items: Vec<Value> = networks
        .into_iter()
        .map(|n| {
            json!({
                "id": n.id.unwrap_or_default(),
                "name": n.name.unwrap_or_default(),
                "driver": n.driver.unwrap_or_default(),
                "scope": n.scope.unwrap_or_default(),
                "internal": n.internal.unwrap_or(false),
            })
        })
        .collect();

    Ok(Json(items))
}

/// `GET /api/docker/configs` — list Docker configs
///
/// Falls back to empty list if Docker CLI is not available or Swarm is not active.
pub async fn list_configs(
    State(_state): State<AppState>,
) -> Result<Json<Vec<Value>>, (StatusCode, String)> {
    let output = match tokio::process::Command::new("docker")
        .args(["config", "ls", "--format", "{{json .}}"])
        .output()
        .await
    {
        Ok(o) => o,
        Err(_) => {
            // docker CLI not available in container
            return Ok(Json(Vec::new()));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // swarm mode might not be active — return empty list gracefully
        if stderr.contains("is not a swarm manager")
            || stderr.contains("This node is not a swarm manager")
        {
            return Ok(Json(Vec::new()));
        }
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("docker config ls failed: {stderr}"),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut items = Vec::new();

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(v) => {
                // Normalise fields to lower-case snake_case keys expected by the API
                let id = v
                    .get("ID")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let name = v
                    .get("Name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let created_at = v
                    .get("CreatedAt")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let updated_at = v
                    .get("UpdatedAt")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                items.push(json!({
                    "id": id,
                    "name": name,
                    "created_at": created_at,
                    "updated_at": updated_at,
                }));
            }
            Err(e) => {
                tracing::warn!("Failed to parse docker config ls line: {e} — {line}");
            }
        }
    }

    Ok(Json(items))
}

/// `GET /api/docker/secrets` — list Docker secrets via Bollard
pub async fn list_secrets(
    State(state): State<AppState>,
) -> Result<Json<Vec<Value>>, (StatusCode, String)> {
    let result = state.docker.list_secrets::<String>(None).await;

    let secrets = match result {
        Ok(s) => s,
        Err(e) => {
            let msg = e.to_string();
            // swarm mode might not be active — return empty list gracefully
            if msg.contains("Swarm does not have a leader")
                || msg.contains("is not a swarm manager")
                || msg.contains("This node is not a swarm manager")
            {
                return Ok(Json(Vec::new()));
            }
            return Err((StatusCode::INTERNAL_SERVER_ERROR, msg));
        }
    };

    let items: Vec<Value> = secrets
        .into_iter()
        .map(|s| {
            let created_at = s.created_at.map(|d| d.to_string()).unwrap_or_default();
            let updated_at = s.updated_at.map(|d| d.to_string()).unwrap_or_default();
            let name = s
                .spec
                .as_ref()
                .and_then(|spec| spec.name.clone())
                .unwrap_or_default();
            json!({
                "id": s.id.unwrap_or_default(),
                "name": name,
                "created_at": created_at,
                "updated_at": updated_at,
            })
        })
        .collect();

    Ok(Json(items))
}

/// Mount all docker-resource routes under `/api/docker/*`
pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/api/docker/volumes", axum::routing::get(list_volumes))
        .route("/api/docker/networks", axum::routing::get(list_networks))
        .route("/api/docker/configs", axum::routing::get(list_configs))
        .route("/api/docker/secrets", axum::routing::get(list_secrets))
}
