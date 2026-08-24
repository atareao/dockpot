use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;

use crate::auth::AppState;

pub async fn logs_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_logs_socket(socket, state, id))
}

async fn handle_logs_socket(mut socket: WebSocket, state: Arc<AppState>, id: String) {
    let stack = match state.db.get_stack(&id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            let _ = socket
                .send(Message::Text("❌ Stack not found".into()))
                .await;
            return;
        }
        Err(e) => {
            let _ = socket
                .send(Message::Text(format!("❌ DB error: {}", e).into()))
                .await;
            return;
        }
    };

    let compose_path = format!("{}/compose.yaml", stack.path);

    // Spawn docker compose logs -f
    let mut child = match Command::new("docker")
        .args([
            "compose",
            "-f",
            &compose_path,
            "logs",
            "-f",
            "--tail=100",
            "--no-color",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = socket
                .send(Message::Text(
                    format!("❌ Failed to spawn docker: {}", e).into(),
                ))
                .await;
            return;
        }
    };

    let (stdout, stderr) = match (child.stdout.take(), child.stderr.take()) {
        (Some(o), Some(e)) => (o, e),
        _ => {
            let _ = socket
                .send(Message::Text("❌ Failed to capture output".into()))
                .await;
            return;
        }
    };

    let (mut ws_tx, mut ws_rx) = socket.split();

    // Send initial message
    let _ = ws_tx
        .send(Message::Text(
            format!("📋 Connecting to logs for '{}'...\n", stack.name).into(),
        ))
        .await;

    // Channel to bridge stdout/stderr to WebSocket
    let (log_tx, mut log_rx) = tokio::sync::mpsc::channel::<String>(256);

    // Read stdout task
    let tx1 = log_tx.clone();
    let stdout_task = tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    let msg = line.trim_end().to_string();
                    if tx1.send(msg).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Read stderr task
    let tx2 = log_tx;
    let stderr_task = tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stderr);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    let msg = format!("⚠️ {}", line.trim_end());
                    if tx2.send(msg).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Forward logs from channel to WebSocket, or handle close
    let ws_to_log = tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(msg) = log_rx.recv() => {
                    if ws_tx.send(Message::Text(msg.into())).await.is_err() {
                        break;
                    }
                }
                ws_msg = ws_rx.next() => {
                    match ws_msg {
                        Some(Ok(Message::Close(_))) | None => break,
                        Some(Ok(Message::Ping(data))) => {
                            let _ = ws_tx.send(Message::Pong(data)).await;
                        }
                        Some(Err(_)) => break,
                        _ => {}
                    }
                }
            }
        }
    });

    // Wait for everything to finish
    let _ = tokio::join!(stdout_task, stderr_task, ws_to_log);

    // Cleanup
    let _ = child.kill().await;
    let _ = child.wait().await;
    tracing::info!("🔌 Logs WebSocket closed for stack '{}'", stack.name);
}
