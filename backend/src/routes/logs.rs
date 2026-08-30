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

/// SSE handler for streaming Docker container logs of a stack.
///
/// The `id` path parameter is a **stack ID** (UUID). The handler looks up
/// the stack in the database, resolves its name, and uses that name as the
/// Docker container name for `docker.logs()`.
///
/// # Current limitation (temporal adaptation)
///
/// Bollard requires a specific container name, not a compose project name.
/// For stacks deployed via Docker Compose, containers are typically named
/// `{project}_{service}_{N}`.  This handler currently uses **the stack name
/// directly** as the container name, which only works when the stack name
/// happens to match a running container (e.g. single-service stacks).
///
/// Future work: resolve all containers belonging to a compose project and
/// multiplex their log streams into a single SSE connection.
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

    // ── Configure Bollard log stream ─────────────────────────────────
    //
    // NOTE: The stack name is used as the container name.  This works for
    // stacks that match a running container name directly.  Compose-project
    // multiplexing will be added in a future iteration.
    let options = LogsOptions::<String> {
        follow: true,
        stdout: true,
        stderr: true,
        tail: "100".to_string(),
        ..Default::default()
    };

    let stream = docker.logs(&stack.name, Some(options));
    let mut stream = Box::pin(stream);

    // ── Bridge Bollard stream → SSE channel ─────────────────────────
    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(100);

    tokio::spawn(async move {
        while let Some(item) = stream.next().await {
            match item {
                Ok(LogOutput::StdOut { message }) | Ok(LogOutput::StdErr { message }) => {
                    let text = String::from_utf8_lossy(&message).to_string();
                    if tx
                        .send(Ok(Event::default().event("log").data(text)))
                        .await
                        .is_err()
                    {
                        // Receiver dropped (client disconnected)
                        break;
                    }
                }
                Ok(_) => {
                    // Other log output types (e.g. console) — skip
                    continue;
                }
                Err(e) => {
                    let _ = tx
                        .send(Ok(Event::default()
                            .event("error")
                            .data(format!("Docker error: {e}"))))
                        .await;
                    break;
                }
            }
        }
    });

    Ok(Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default()))
}
