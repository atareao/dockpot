use axum::{
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
};
use bollard::{container::LogOutput, container::LogsOptions, Docker};
use futures::StreamExt;
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::db::Database;
use crate::models::AppError;

/// Resolve the actual running container names for a compose project
/// by parsing `docker compose ps --format '{{.Name}}'` output.
async fn resolve_compose_containers(stack_path: &str) -> Vec<String> {
    let compose_path = format!("{stack_path}/compose.yaml");
    let output = tokio::process::Command::new("docker")
        .args([
            "compose",
            "-f",
            &compose_path,
            "ps",
            "--format",
            "{{.Name}}",
        ])
        .output()
        .await;
    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        }
        _ => vec![],
    }
}

/// SSE handler for streaming Docker container logs of a stack.
///
/// Resolves the actual container names from the compose project (via
/// `docker compose ps`) and streams logs from all of them multiplexed
/// into a single SSE connection.
pub async fn logs_sse_handler(
    State(docker): State<Docker>,
    State(db): State<Database>,
    Path(id): Path<String>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, AppError> {
    // ── Look up stack in DB ──────────────────────────────────────────
    let stack = db
        .get_stack(&id)
        .await
        .map_err(|e| AppError::Internal(format!("Database error looking up stack: {e}")))?
        .ok_or_else(|| AppError::NotFound(format!("Stack '{id}' not found")))?;

    // ── Resolve container names from compose project ─────────────────
    let container_names = resolve_compose_containers(&stack.path).await;

    if container_names.is_empty() {
        return Err(AppError::NotFound(format!(
            "No running containers found for stack '{}'",
            stack.name
        )));
    }

    tracing::debug!(
        "Streaming logs for '{}' from containers: {:?}",
        stack.name,
        container_names
    );

    // ── Configure Bollard log stream options ─────────────────────────
    let options = LogsOptions::<String> {
        follow: true,
        stdout: true,
        stderr: true,
        tail: "100".to_string(),
        ..Default::default()
    };

    // ── Bridge Bollard stream → SSE channel ─────────────────────────
    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(100);

    tokio::spawn(async move {
        // Stream logs from ALL containers, multiplexed
        for container_name in &container_names {
            let tx = tx.clone();
            let name = container_name.clone();
            let opts = options.clone();
            let d = docker.clone();

            tokio::spawn(async move {
                let mut stream = Box::pin(d.logs(&name, Some(opts)));
                while let Some(item) = stream.next().await {
                    match item {
                        Ok(LogOutput::StdOut { message }) | Ok(LogOutput::StdErr { message }) => {
                            let text = String::from_utf8_lossy(&message).to_string();
                            let prefixed = format!("[{name}] {text}");
                            if tx
                                .send(Ok(Event::default().event("log").data(prefixed)))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Ok(_) => continue,
                        Err(e) => {
                            let _ = tx
                                .send(Ok(Event::default()
                                    .event("error")
                                    .data(format!("[{name}] Docker error: {e}"))))
                                .await;
                            break;
                        }
                    }
                }
            });
        }
    });

    Ok(Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default()))
}
