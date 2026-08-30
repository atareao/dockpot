use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

use crate::git;
use crate::models::{CreateStackRequest, UpdateStackRequest};
use crate::state::AppState;

/// Auto-commit changes if the stack has git sync enabled
async fn auto_commit(state: &AppState, id: &str, message: &str) {
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

/// Raw project from `docker compose ls --format json`
#[allow(non_snake_case)]
#[derive(Deserialize)]
struct ComposeProject {
    Name: String,
    Status: String,
    ConfigFiles: String,
}

/// Body for the import endpoint
#[derive(Deserialize)]
pub struct ImportStackRequest {
    pub name: String,
}

/// Run `docker compose ls --format json` and return the parsed projects.
async fn discover_projects() -> Result<Vec<ComposeProject>, String> {
    let output = tokio::process::Command::new("docker")
        .args(["compose", "ls", "--format", "json"])
        .output()
        .await
        .map_err(|e| format!("Failed to run docker compose ls: {e}"))?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<Vec<ComposeProject>>(&stdout)
        .map_err(|e| format!("Failed to parse compose ls output: {e}"))
}

pub async fn discover(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, String)> {
    let projects = discover_projects().await.map_err(|e| {
        tracing::error!("Failed to discover projects: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let _managed_stacks = state.db.list_stacks().await.map_err(|e| {
        tracing::error!("Failed to list stacks: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let mut results = Vec::new();
    for project in &projects {
        let managed = project.ConfigFiles.contains("/app/stacks/");
        let compose = if managed {
            None
        } else {
            // Try reading the file directly first, fall back to `docker compose config`
            match tokio::fs::read_to_string(&project.ConfigFiles).await {
                Ok(c) => Some(c),
                Err(_) => {
                    // File not directly accessible (e.g. host path) — use docker compose config
                    let config_path = project.ConfigFiles.trim();
                    let output = tokio::process::Command::new("docker")
                        .args(["compose", "-f", config_path, "config"])
                        .output()
                        .await
                        .ok();
                    output.and_then(|o| {
                        if o.status.success() {
                            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                            if s.is_empty() {
                                None
                            } else {
                                Some(s)
                            }
                        } else {
                            None
                        }
                    })
                }
            }
        };

        results.push(serde_json::json!({
            "name": project.Name,
            "status": project.Status,
            "config_files": project.ConfigFiles,
            "compose": compose,
            "managed": managed,
        }));
    }

    // ── Standalone containers (docker run, no compose) ──
    let container_output = tokio::process::Command::new("docker")
        .args([
            "container",
            "ls",
            "--format",
            "{{.ID}}\t{{.Image}}\t{{.Names}}\t{{.Status}}\t{{.Ports}}\t{{.Labels}}",
        ])
        .output()
        .await
        .ok();

    if let Some(out) = container_output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() < 6 {
                    continue;
                }
                let cid = parts[0];
                let image = parts[1];
                let cname = parts[2];
                let status = parts[3];
                let ports = parts[4];
                let labels = parts[5];

                // Skip if part of a compose project (has com.docker.compose.project label)
                if labels.contains("com.docker.compose.project") {
                    continue;
                }

                results.push(serde_json::json!({
                    "name": cname,
                    "status": status,
                    "image": image,
                    "ports": ports,
                    "container_id": cid,
                    "type": "container",
                    "managed": false,
                }));
            }
        }
    }

    Ok(Json(serde_json::json!(results)))
}

/// Body for create-from-container endpoint
#[derive(Deserialize)]
pub struct CreateFromContainerRequest {
    pub container_name: String,
}

/// Create a compose stack from a running standalone container
pub async fn create_from_container(
    State(state): State<AppState>,
    Json(req): Json<CreateFromContainerRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let name = req.container_name.trim().to_string();
    if name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "container_name is required".into()));
    }

    // Inspect the container
    let output = tokio::process::Command::new("docker")
        .args(["container", "inspect", &name])
        .output()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to inspect container: {e}"),
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err((
            StatusCode::NOT_FOUND,
            format!("Container '{}' not found: {stderr}", name),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let inspect: Value = serde_json::from_str(&stdout).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse inspect: {e}"),
        )
    })?;

    let cfg = &inspect[0]["Config"];
    let host = &inspect[0]["HostConfig"];
    let image = cfg["Image"].as_str().unwrap_or("unknown");

    // Build compose YAML
    let mut compose = format!("services:\n  {}:\n    image: {}\n", name, image);

    // Ports
    let mut port_lines: Vec<String> = Vec::new();
    if let Some(ports) = host["PortBindings"].as_object() {
        for (container_port, bindings) in ports {
            if let Some(arr) = bindings.as_array() {
                for b in arr {
                    let host_port = b["HostPort"].as_str().unwrap_or("");
                    let host_ip = b["HostIp"].as_str().unwrap_or("");
                    let cp = container_port
                        .trim_end_matches("/tcp")
                        .trim_end_matches("/udp");
                    if host_ip.is_empty() {
                        port_lines.push(format!("      - \"{}:{}\"", host_port, cp));
                    } else {
                        port_lines.push(format!("      - \"{}:{}:{}\"", host_ip, host_port, cp));
                    }
                }
            }
        }
    } else {
        // Fallback to exposed ports
        if let Some(exposed) = cfg["ExposedPorts"].as_object() {
            for ep in exposed.keys() {
                port_lines.push(format!("      - \"{}\"", ep));
            }
        }
    }
    if !port_lines.is_empty() {
        compose.push_str("    ports:\n");
        for l in &port_lines {
            compose.push_str(l);
            compose.push('\n');
        }
    }

    // Volumes
    let mut vol_lines: Vec<String> = Vec::new();
    if let Some(mounts) = inspect[0]["Mounts"].as_array() {
        for m in mounts {
            let src = m["Source"].as_str().unwrap_or("");
            let dst = m["Destination"].as_str().unwrap_or("");
            let mode = m["Mode"].as_str().unwrap_or("rw");
            if !src.is_empty() && !dst.is_empty() {
                vol_lines.push(format!("      - \"{}:{}:{}\"", src, dst, mode));
            }
        }
    }
    if !vol_lines.is_empty() {
        compose.push_str("    volumes:\n");
        for l in &vol_lines {
            compose.push_str(l);
            compose.push('\n');
        }
    }

    // Environment
    let mut env_lines: Vec<String> = Vec::new();
    if let Some(env) = cfg["Env"].as_array() {
        for e in env {
            if let Some(s) = e.as_str() {
                env_lines.push(format!("      - {}", s));
            }
        }
    }
    if !env_lines.is_empty() {
        compose.push_str("    environment:\n");
        for l in &env_lines {
            compose.push_str(l);
            compose.push('\n');
        }
    }

    compose.push_str("    restart: unless-stopped\n");

    // Check name uniqueness
    if let Ok(Some(_)) = state.db.get_stack_by_name(&name).await {
        return Err((
            StatusCode::CONFLICT,
            format!("Stack '{}' already exists", name),
        ));
    }

    let stack = state
        .db
        .create_stack(&name, None, &compose)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create stack from container: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    tracing::info!(
        "📦 Stack '{}' created from container '{}'",
        stack.name,
        name
    );

    auto_commit(
        &state,
        &stack.id,
        &format!("dockpot: create stack '{}' from container", stack.name),
    )
    .await;

    Ok(Json(serde_json::json!(stack)))
}

pub async fn import(
    State(state): State<AppState>,
    Json(req): Json<ImportStackRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Name is required".into()));
    }

    let projects = discover_projects().await.map_err(|e| {
        tracing::error!("Failed to discover projects: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let project = projects.iter().find(|p| p.Name == name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("Project '{}' not found in active compose projects", name),
        )
    })?;

    let compose = match tokio::fs::read_to_string(&project.ConfigFiles).await {
        Ok(c) => c,
        Err(_) => {
            // File not accessible (host path) — use docker compose config
            let config_path = project.ConfigFiles.trim();
            let output = tokio::process::Command::new("docker")
                .args(["compose", "-f", config_path, "config"])
                .output()
                .await
                .map_err(|e| {
                    tracing::error!("Failed to run docker compose config: {e}");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to read compose file: {e}"),
                    )
                })?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to read compose file: {stderr}"),
                ));
            }
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
    };

    let stack = state
        .db
        .create_stack(&name, None, &compose)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create stack: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    tracing::info!(
        "📥 Stack '{}' imported from {}",
        stack.name,
        project.ConfigFiles
    );

    auto_commit(
        &state,
        &stack.id,
        &format!("dockpot: import stack '{}'", stack.name),
    )
    .await;

    Ok(Json(serde_json::json!(stack)))
}

pub async fn list(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, String)> {
    let stacks = state.db.list_stacks().await.map_err(|e| {
        tracing::error!("Failed to list stacks: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok(Json(serde_json::json!(stacks)))
}

pub async fn create(
    State(state): State<AppState>,
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
    State(state): State<AppState>,
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
    State(state): State<AppState>,
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
    State(state): State<AppState>,
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
    State(state): State<AppState>,
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
    State(state): State<AppState>,
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
    State(state): State<AppState>,
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
    State(state): State<AppState>,
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
    State(state): State<AppState>,
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
    State(_state): State<AppState>,
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
    State(state): State<AppState>,
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
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let status = state.db.get_dashboard_status().await.map_err(|e| {
        tracing::error!("Failed to get dashboard status: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok(Json(serde_json::json!(status)))
}
