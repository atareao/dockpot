use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use axum::http::StatusCode;
use serde_json::Value;

use crate::auth::AppState;

pub async fn docker_info(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let version = tokio::process::Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .output().await.ok()
        .and_then(|o| {
            let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if v.is_empty() { None } else { Some(v) }
        });

    let engine = tokio::process::Command::new("docker")
        .args(["info", "--format", "{{.ServerVersion}}"])
        .output().await.ok()
        .and_then(|o| {
            let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if v.is_empty() { None } else { Some(v) }
        });

    let containers_total = tokio::process::Command::new("docker")
        .args(["ps", "-aq"])
        .output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().filter(|l| !l.is_empty()).count() as u64)
        .unwrap_or(0);

    let containers_running = tokio::process::Command::new("docker")
        .args(["ps", "-q"])
        .output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().filter(|l| !l.is_empty()).count() as u64)
        .unwrap_or(0);

    let images = tokio::process::Command::new("docker")
        .args(["images", "-q"])
        .output().await.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().filter(|l| !l.is_empty()).count() as u64)
        .unwrap_or(0);

    let disk_usage = tokio::process::Command::new("docker")
        .args(["system", "df", "--format", "{{.Size}}"])
        .output().await.ok()
        .and_then(|o| {
            let v = String::from_utf8_lossy(&o.stdout).lines().next()?.to_string();
            if v.is_empty() { None } else { Some(v) }
        });

    Ok(Json(serde_json::json!({
        "version": version,
        "engine": engine,
        "containers_total": containers_total,
        "containers_running": containers_running,
        "images": images,
        "disk_usage": disk_usage,
    })))
}