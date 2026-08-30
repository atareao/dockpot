use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;

use crate::state::AppState;

pub async fn export_zip(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    let stack = state
        .db
        .get_stack(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Stack '{}' not found", id)))?;

    // Create a temp dir and zip the stack files
    let tmp_dir = std::env::temp_dir().join(format!("dockpot-export-{}", id));
    let _ = tokio::fs::create_dir_all(&tmp_dir).await;

    // Copy compose.yaml
    let compose_src = std::path::Path::new(&stack.path).join("compose.yaml");
    let compose_dst = tmp_dir.join("compose.yaml");
    if compose_src.exists() {
        tokio::fs::copy(&compose_src, &compose_dst).await.ok();
    }

    // Copy .env files
    if let Ok(envs) = state.db.list_env_files(&id).await {
        for env in envs {
            let env_path = tmp_dir.join(&env.filename);
            tokio::fs::write(&env_path, &env.content).await.ok();
        }
    }

    // Copy git if exists
    let git_src = std::path::Path::new(&stack.path).join(".git");
    let git_dst = tmp_dir.join(".git");
    if git_src.exists() {
        let _ = std::process::Command::new("cp")
            .args([
                "-r",
                git_src.to_str().unwrap_or(""),
                git_dst.to_str().unwrap_or(""),
            ])
            .output();
    }

    // Create zip
    let zip_path = std::env::temp_dir().join(format!("dockpot-{}.zip", stack.name));
    let _ = std::process::Command::new("zip")
        .args(["-r", zip_path.to_str().unwrap_or(""), "."])
        .current_dir(&tmp_dir)
        .output();

    // Read zip
    let data = tokio::fs::read(&zip_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read zip: {}", e),
        )
    })?;

    // Cleanup temp
    let _ = tokio::fs::remove_dir_all(&tmp_dir).await;
    let _ = tokio::fs::remove_file(&zip_path).await;

    let filename = format!("{}-stack.zip", stack.name);
    let response = Response::builder()
        .header("Content-Type", "application/zip")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(data))
        .unwrap();

    Ok(response)
}
