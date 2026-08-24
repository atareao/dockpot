use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::Value;
use uuid::Uuid;

use crate::auth::AppState;
use crate::models::{Agent, CreateAgentRequest};

pub async fn list(State(state): State<Arc<AppState>>) -> Result<Json<Value>, (StatusCode, String)> {
    let agents = state.db.list_agents().await.map_err(|e| {
        tracing::error!("Failed to list agents: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok(Json(serde_json::json!(agents)))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let agent = state
        .db
        .get_agent(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get agent: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Agent '{}' not found", id)))?;

    Ok(Json(serde_json::json!(agent)))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    if req.name.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Name is required".into()));
    }
    if req.host.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Host is required".into()));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let agent = Agent {
        id: Uuid::new_v4().to_string(),
        name: req.name,
        agent_type: "docker".into(),
        host: req.host,
        port: req.port.unwrap_or(2376),
        tls_enabled: req.tls_enabled.unwrap_or(true),
        ca_cert: req.ca_cert,
        client_cert: req.client_cert,
        client_key: req.client_key,
        description: req.description,
        enabled: true,
        created_at: now.clone(),
        updated_at: now,
    };

    state.db.create_agent(&agent).await.map_err(|e| {
        tracing::error!("Failed to create agent: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    tracing::info!("✅ Agent '{}' created", agent.name);
    Ok(Json(serde_json::json!(agent)))
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let existing = state
        .db
        .get_agent(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get agent: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Agent '{}' not found", id)))?;

    let agent = Agent {
        id: id.clone(),
        name: req.name,
        agent_type: existing.agent_type,
        host: req.host,
        port: req.port.unwrap_or(existing.port),
        tls_enabled: req.tls_enabled.unwrap_or(existing.tls_enabled),
        ca_cert: req.ca_cert.or(existing.ca_cert),
        client_cert: req.client_cert.or(existing.client_cert),
        client_key: req.client_key.or(existing.client_key),
        description: req.description.or(existing.description),
        enabled: existing.enabled,
        created_at: existing.created_at,
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    state.db.update_agent(&id, &agent).await.map_err(|e| {
        tracing::error!("Failed to update agent: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    tracing::info!("📝 Agent '{}' updated", agent.name);
    Ok(Json(serde_json::json!(agent)))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let deleted = state.db.delete_agent(&id).await.map_err(|e| {
        tracing::error!("Failed to delete agent: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    if !deleted {
        return Err((StatusCode::NOT_FOUND, format!("Agent '{}' not found", id)));
    }

    Ok(StatusCode::NO_CONTENT)
}
