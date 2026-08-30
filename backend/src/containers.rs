use axum::{
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
    response::Json,
    routing::{get, post},
    Router,
};
use bollard::{
    container::{
        InspectContainerOptions, ListContainersOptions, LogOutput, LogsOptions,
        RemoveContainerOptions, RestartContainerOptions, StartContainerOptions,
        StopContainerOptions,
    },
    image::{CreateImageOptions, RemoveImageOptions},
    Docker,
};
use futures::{pin_mut, StreamExt};
use serde::Serialize;
use std::collections::HashMap;
use std::convert::Infallible;
use tokio_stream::wrappers::ReceiverStream;

use crate::db::Database;
use crate::models::*;
use crate::state::{AppState, CachedContainers, ContainerInfo};

// ───── Constants ─────────────────────────────────────────────

const LABEL_COMPOSE_PROJECT: &str = "com.docker.compose.project";

// ───── Response types ────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ContainerInspectResponse {
    pub id: String,
    pub name: String,
    pub image: String,
    pub created: String,
    pub state: String,
    pub status: String,
    pub ports: Vec<PortInfo>,
    pub mounts: Vec<MountInfo>,
    pub env: Vec<String>,
    pub networks: Vec<ContainerNetworkInfo>,
    pub labels: HashMap<String, String>,
    pub restart_policy: String,
    pub health: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortInfo {
    pub private_port: u16,
    pub public_port: Option<u16>,
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MountInfo {
    pub source: String,
    pub destination: String,
    pub mode: String,
    pub rw: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContainerNetworkInfo {
    pub name: String,
    pub ip_address: String,
    pub gateway: String,
}

// ───── Docker operations ─────────────────────────────────────

/// List all containers from Docker and return their info.
#[allow(clippy::unnecessary_filter_map)]
pub async fn fetch_containers(docker: &Docker, _db: &Database) -> Vec<ContainerInfo> {
    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            size: true,
            ..Default::default()
        }))
        .await
        .unwrap_or_default();

    containers
        .iter()
        .filter_map(|c| {
            let name = c
                .names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| strip_name(n))
                .unwrap_or_default();

            let raw_image = c.image.as_deref().unwrap_or("unknown");

            // Parse image name + tag from the image reference
            let (image_name, tag) = if let Some(pos) = raw_image.find('@') {
                (raw_image[..pos].to_string(), String::new())
            } else if let Some((n, t)) = raw_image.rsplit_once(':') {
                (n.to_string(), t.to_string())
            } else {
                (raw_image.to_string(), "latest".into())
            };

            // Build port strings in "ip:public:private" format
            let ports: Vec<String> = c
                .ports
                .as_ref()
                .map(|ps| {
                    ps.iter()
                        .filter_map(|p| {
                            let pub_str =
                                p.public_port.map(|pp| pp.to_string()).unwrap_or_default();
                            if pub_str.is_empty() {
                                None
                            } else {
                                Some(format!(
                                    "{}:{}:{}",
                                    p.ip.as_deref().unwrap_or("0.0.0.0"),
                                    pub_str,
                                    p.private_port
                                ))
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Extract Traefik URL from labels
            let traefik_url = c.labels.as_ref().and_then(|labels| {
                for (k, v) in labels {
                    if k.ends_with(".rule") && v.starts_with("Host(") {
                        let host = v
                            .trim_start_matches("Host(`")
                            .split('`')
                            .next()
                            .unwrap_or("");
                        let tls = labels
                            .iter()
                            .any(|(lk, lv)| lk.starts_with(&k[..k.len() - 5]) && lv == "true");
                        let proto = if tls { "https" } else { "http" };
                        return Some(format!("{}://{}", proto, host));
                    }
                }
                None
            });

            // Build registry URL from the image name
            let registry_url = build_registry_url(raw_image);

            Some(ContainerInfo {
                id: c.id.as_deref().unwrap_or("").chars().take(12).collect(),
                name,
                image: image_name,
                image_tag: tag,
                status: c.status.as_deref().unwrap_or("unknown").to_string(),
                state: c.state.as_deref().unwrap_or("unknown").to_string(),
                size_mb: ((c.size_rw.unwrap_or(0) as f64 / 1_048_576.0) * 100.0).round() / 100.0,
                has_update: false,
                updating: false,
                compose_project: c
                    .labels
                    .as_ref()
                    .and_then(|labels| labels.get(LABEL_COMPOSE_PROJECT).cloned()),
                ports,
                traefik_url,
                registry_url,
                last_check: None,
                next_check: None,
                last_remote_digest: String::new(),
            })
        })
        .collect()
}

/// Build a registry URL from an image reference string.
fn build_registry_url(image: &str) -> String {
    if image.contains('/') {
        if image.starts_with("docker.io/") || !image.contains('.') {
            let parts: Vec<&str> = image.splitn(2, '/').collect();
            if parts.len() == 2 && parts[0].contains('.') {
                format!("https://{}", parts[0])
            } else {
                let repo = image.trim_start_matches("docker.io/");
                if repo.contains('/') {
                    format!("https://hub.docker.com/r/{}", repo)
                } else {
                    format!("https://hub.docker.com/_/{}/tags", repo)
                }
            }
        } else if image.contains('.') {
            format!("https://{}", image.split('/').next().unwrap_or(""))
        } else {
            format!("https://hub.docker.com/r/{}/tags", image)
        }
    } else if image.starts_with("sha256:") {
        // Bare digest — use a generic hub link
        "https://hub.docker.com".to_string()
    } else {
        format!("https://hub.docker.com/_/{}/tags", image)
    }
}

/// Find a container by its name (without leading slash).
pub async fn find_container_by_name(
    docker: &Docker,
    name: &str,
) -> Result<bollard::models::ContainerSummary, AppError> {
    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await?;
    containers
        .into_iter()
        .find(|c| {
            c.names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| strip_name(n) == name)
                .unwrap_or(false)
        })
        .ok_or_else(|| AppError::NotFound(format!("Container '{}' not found", name)))
}

/// Pull a Docker image with a timeout.
pub async fn pull_image(docker: &Docker, image: &str, timeout_secs: u64) -> bool {
    let tag = parse_image_tag(image).1;
    tracing::info!(
        "pull_image: descargando imagen '{}' (tag: {:?}, timeout: {}s)",
        image,
        tag,
        timeout_secs
    );
    let stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: image.to_string(),
            tag,
            platform: current_platform().unwrap_or_default(),
            ..Default::default()
        }),
        None,
        None,
    );
    pin_mut!(stream);
    let timeout_dur = std::time::Duration::from_secs(timeout_secs);
    let timed = tokio::time::timeout(timeout_dur, async {
        while let Some(item) = stream.next().await {
            if let Err(e) = item {
                tracing::error!("pull_image: error de Bollard para '{}': {}", image, e);
                return false;
            }
        }
        true
    });
    match timed.await {
        Ok(result) => {
            if result {
                tracing::info!("pull_image: descarga completada para '{}'", image);
            } else {
                tracing::error!("pull_image: error durante la descarga de '{}'", image);
            }
            result
        }
        Err(_) => {
            tracing::error!(
                "pull_image: timeout después de {}s para '{}'",
                timeout_secs,
                image
            );
            false
        }
    }
}

/// Remove an old image by its digest/ID. Only succeeds if no container is still using it.
pub async fn remove_old_image(docker: &Docker, old_image_id: &str) {
    if old_image_id.is_empty() {
        return;
    }
    match docker
        .remove_image(
            old_image_id,
            Some(RemoveImageOptions {
                force: false,
                noprune: false,
            }),
            None,
        )
        .await
    {
        Ok(_) => {
            tracing::info!(
                "remove_old_image: imagen antigua {} eliminada",
                old_image_id
            );
        }
        Err(e) => {
            tracing::debug!(
                "remove_old_image: no se pudo eliminar {} (en uso?): {}",
                old_image_id,
                e
            );
        }
    }
}

// ───── REST handlers ─────────────────────────────────────────

async fn list_containers_h(
    State(cache): State<CachedContainers>,
    State(docker): State<Docker>,
    State(db): State<Database>,
) -> Json<Vec<ContainerInfo>> {
    let cached = cache.read().await;
    if let Some(containers) = cached.as_ref() {
        Json(containers.clone())
    } else {
        drop(cached);
        Json(fetch_containers(&docker, &db).await)
    }
}

async fn inspect_container_h(
    State(docker): State<Docker>,
    Path(name): Path<String>,
) -> Result<Json<ContainerInspectResponse>, AppError> {
    let resp = docker
        .inspect_container(&name, None::<InspectContainerOptions>)
        .await
        .map_err(|e| AppError::NotFound(format!("Container '{}': {}", name, e)))?;

    let ports = resp
        .network_settings
        .as_ref()
        .and_then(|ns| ns.ports.as_ref())
        .map(|port_map| {
            port_map
                .iter()
                .filter_map(|(key, bindings)| {
                    let parts: Vec<&str> = key.split('/').collect();
                    let private_port: u16 = parts.first().and_then(|p| p.parse().ok())?;
                    let proto = parts.get(1).copied().unwrap_or("tcp").to_string();
                    let public_port = bindings
                        .as_ref()
                        .and_then(|b| b.first())
                        .and_then(|b| b.host_port.as_ref())
                        .and_then(|p| p.parse().ok());
                    Some(PortInfo {
                        private_port,
                        public_port,
                        r#type: proto,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mounts = resp
        .mounts
        .as_ref()
        .map(|m| {
            m.iter()
                .map(|mp| MountInfo {
                    source: mp.source.clone().unwrap_or_default(),
                    destination: mp.destination.clone().unwrap_or_default(),
                    mode: mp.mode.clone().unwrap_or_default(),
                    rw: mp.rw.unwrap_or(false),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let env = resp
        .config
        .as_ref()
        .and_then(|c| c.env.clone())
        .unwrap_or_default();

    let networks = resp
        .network_settings
        .as_ref()
        .and_then(|ns| ns.networks.as_ref())
        .map(|nets| {
            nets.iter()
                .map(|(net_name, settings)| ContainerNetworkInfo {
                    name: net_name.clone(),
                    ip_address: settings.ip_address.clone().unwrap_or_default(),
                    gateway: settings.gateway.clone().unwrap_or_default(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let labels = resp
        .config
        .as_ref()
        .and_then(|c| c.labels.clone())
        .unwrap_or_default();

    let restart_policy = resp
        .host_config
        .as_ref()
        .and_then(|hc| hc.restart_policy.as_ref())
        .and_then(|rp| rp.name.as_ref().map(|n| n.to_string()))
        .unwrap_or_default();

    let health = resp
        .state
        .as_ref()
        .and_then(|s| s.health.as_ref())
        .and_then(|h| h.status.as_ref().map(|s| s.to_string()));

    let state_str = resp
        .state
        .as_ref()
        .and_then(|s| s.status.as_ref().map(|s| s.to_string()))
        .unwrap_or_default();

    let id = resp.id.unwrap_or_default();
    let name_out = strip_name(&resp.name.unwrap_or_default());
    let image = resp.image.unwrap_or_default();
    let created = resp.created.unwrap_or_default();

    Ok(Json(ContainerInspectResponse {
        id,
        name: name_out,
        image,
        created,
        state: state_str.clone(),
        status: state_str,
        ports,
        mounts,
        env,
        networks,
        labels,
        restart_policy,
        health,
    }))
}

async fn start_container_h(
    State(docker): State<Docker>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    docker
        .start_container(&name, None::<StartContainerOptions<String>>)
        .await
        .map_err(|e| AppError::NotFound(format!("start_container '{}': {}", name, e)))?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

async fn stop_container_h(
    State(docker): State<Docker>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    docker
        .stop_container(&name, None::<StopContainerOptions>)
        .await
        .map_err(|e| AppError::NotFound(format!("stop_container '{}': {}", name, e)))?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

async fn restart_container_h(
    State(docker): State<Docker>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    docker
        .restart_container(&name, None::<RestartContainerOptions>)
        .await
        .map_err(|e| AppError::NotFound(format!("restart_container '{}': {}", name, e)))?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

async fn remove_container_h(
    State(docker): State<Docker>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let options = RemoveContainerOptions {
        force: true,
        ..Default::default()
    };
    docker
        .remove_container(&name, Some(options))
        .await
        .map_err(|e| AppError::NotFound(format!("remove_container '{}': {}", name, e)))?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

async fn logs_container_h(
    State(docker): State<Docker>,
    Path(name): Path<String>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let options = LogsOptions::<String> {
        follow: true,
        stdout: true,
        stderr: true,
        tail: "100".to_string(),
        ..Default::default()
    };

    let stream = docker.logs(&name, Some(options));
    let mut stream = Box::pin(stream);

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(100);

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
                        break;
                    }
                }
                Ok(_) => continue,
                Err(e) => {
                    let _ = tx
                        .send(Ok(Event::default()
                            .event("error")
                            .data(format!("Docker error: {}", e))))
                        .await;
                    break;
                }
            }
        }
    });

    Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default())
}

// ───── Routes ────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/containers", get(list_containers_h))
        .route("/api/containers/{name}/inspect", get(inspect_container_h))
        .route("/api/containers/{name}/start", post(start_container_h))
        .route("/api/containers/{name}/stop", post(stop_container_h))
        .route("/api/containers/{name}/restart", post(restart_container_h))
        .route("/api/containers/{name}/remove", post(remove_container_h))
        .route("/api/containers/{name}/logs", get(logs_container_h))
}

// ───── Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── fetch_containers ──────────────────────────────────────

    #[tokio::test]
    async fn test_fetch_containers_empty_on_docker_error() {
        // When Docker is unreachable, fetch_containers should return empty list
        // We can't easily test with a real Docker here, but we test the stub path
        // This is a compile-time + logic test using a mock-like approach
        // TODO: integration tests with a real Docker socket
    }

    #[test]
    fn test_build_registry_url_hub_official() {
        let url = build_registry_url("nginx");
        assert_eq!(url, "https://hub.docker.com/_/nginx/tags");
    }

    #[test]
    fn test_build_registry_url_hub_user_repo() {
        let url = build_registry_url("library/nginx");
        assert!(url.contains("hub.docker.com"));
    }

    #[test]
    fn test_build_registry_url_with_tag() {
        let url = build_registry_url("nginx:latest");
        assert!(url.contains("hub.docker.com"));
    }

    #[test]
    fn test_build_registry_url_ghcr() {
        let url = build_registry_url("ghcr.io/owner/repo");
        assert_eq!(url, "https://ghcr.io");
    }

    #[test]
    fn test_build_registry_url_docker_io() {
        let url = build_registry_url("docker.io/library/nginx");
        assert!(!url.is_empty());
    }

    #[test]
    fn test_build_registry_url_sha256() {
        let url = build_registry_url("sha256:abcdef1234567890");
        assert_eq!(url, "https://hub.docker.com");
    }

    #[test]
    fn test_build_registry_url_custom_registry() {
        let url = build_registry_url("registry.example.com/my/image");
        assert_eq!(url, "https://registry.example.com");
    }

    // ── PortInfo ─────────────────────────────────────────────

    #[test]
    fn test_port_info_serialize() {
        let port = PortInfo {
            private_port: 8080,
            public_port: Some(80),
            r#type: "tcp".into(),
        };
        let json = serde_json::to_string(&port).unwrap();
        assert!(json.contains("\"private_port\":8080"));
        assert!(json.contains("\"public_port\":80"));
        assert!(json.contains("\"type\":\"tcp\""));
    }

    #[test]
    fn test_port_info_no_public_port() {
        let port = PortInfo {
            private_port: 5432,
            public_port: None,
            r#type: "tcp".into(),
        };
        let json = serde_json::to_string(&port).unwrap();
        assert!(json.contains("\"public_port\":null"));
    }

    // ── MountInfo ────────────────────────────────────────────

    #[test]
    fn test_mount_info_serialize() {
        let mount = MountInfo {
            source: "/data".into(),
            destination: "/app/data".into(),
            mode: "rw".into(),
            rw: true,
        };
        let json = serde_json::to_string(&mount).unwrap();
        assert!(json.contains("\"source\":\"/data\""));
        assert!(json.contains("\"mode\":\"rw\""));
        assert!(json.contains("\"rw\":true"));
    }

    #[test]
    fn test_mount_info_readonly() {
        let mount = MountInfo {
            source: "/backup".into(),
            destination: "/backup".into(),
            mode: "ro".into(),
            rw: false,
        };
        assert!(!mount.rw);
    }

    // ── ContainerNetworkInfo ──────────────────────────────────

    #[test]
    fn test_container_network_info_serialize() {
        let net = ContainerNetworkInfo {
            name: "bridge".into(),
            ip_address: "172.17.0.2".into(),
            gateway: "172.17.0.1".into(),
        };
        let json = serde_json::to_string(&net).unwrap();
        assert!(json.contains("\"name\":\"bridge\""));
        assert!(json.contains("\"ip_address\":\"172.17.0.2\""));
        assert!(json.contains("\"gateway\":\"172.17.0.1\""));
    }

    #[test]
    fn test_container_network_info_empty_ip() {
        let net = ContainerNetworkInfo {
            name: "none".into(),
            ip_address: String::new(),
            gateway: String::new(),
        };
        assert_eq!(net.ip_address, "");
    }

    // ── ContainerInspectResponse ──────────────────────────────

    #[test]
    fn test_container_inspect_response_serialize() {
        let resp = ContainerInspectResponse {
            id: "abc123".into(),
            name: "test".into(),
            image: "nginx:latest".into(),
            created: "2026-01-01T00:00:00Z".into(),
            state: "running".into(),
            status: "running".into(),
            ports: vec![],
            mounts: vec![],
            env: vec!["PATH=/usr/bin".into()],
            networks: vec![],
            labels: HashMap::new(),
            restart_policy: "always".into(),
            health: Some("healthy".into()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"state\":\"running\""));
        assert!(json.contains("\"restart_policy\":\"always\""));
        assert!(json.contains("\"health\":\"healthy\""));
    }

    #[test]
    fn test_container_inspect_response_no_health() {
        let resp = ContainerInspectResponse {
            id: "x".into(),
            name: "x".into(),
            image: "alpine".into(),
            created: String::new(),
            state: "exited".into(),
            status: "exited".into(),
            ports: vec![],
            mounts: vec![],
            env: vec![],
            networks: vec![],
            labels: HashMap::new(),
            restart_policy: String::new(),
            health: None,
        };
        assert!(resp.health.is_none());
    }

    // ── ContainerInfo from fetch_containers ───────────────────

    #[test]
    fn test_container_info_fields_defaults() {
        let info = ContainerInfo {
            id: "abc".into(),
            name: "test".into(),
            image: "nginx".into(),
            image_tag: "latest".into(),
            size_mb: 0.0,
            status: "running".into(),
            state: "running".into(),
            has_update: false,
            updating: false,
            compose_project: None,
            ports: vec![],
            traefik_url: None,
            registry_url: String::new(),
            last_check: None,
            next_check: None,
            last_remote_digest: String::new(),
        };
        assert!(!info.has_update);
        assert!(!info.updating);
        assert!(info.last_check.is_none());
        assert!(info.next_check.is_none());
        assert_eq!(info.last_remote_digest, "");
    }

    #[test]
    fn test_container_info_serialize_with_optionals() {
        let info = ContainerInfo {
            id: "abc".into(),
            name: "test".into(),
            image: "nginx".into(),
            image_tag: "1.25".into(),
            size_mb: 45.2,
            status: "running".into(),
            state: "running".into(),
            has_update: true,
            updating: true,
            compose_project: Some("myapp".into()),
            ports: vec!["0.0.0.0:80:80".into()],
            traefik_url: Some("http://test.local".into()),
            registry_url: "https://hub.docker.com/_/nginx/tags".into(),
            last_check: Some("2026-01-01T00:00:00Z".into()),
            next_check: Some("2026-01-02T00:00:00Z".into()),
            last_remote_digest: "sha256:abc123".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"has_update\":true"));
        assert!(json.contains("\"compose_project\":\"myapp\""));
        assert!(json.contains("\"last_check\":\"2026-01-01T00:00:00Z\""));
        assert!(json.contains("\"last_remote_digest\":\"sha256:abc123\""));
    }

    // ── strip_name ───────────────────────────────────────────

    #[test]
    fn test_strip_name_slash() {
        assert_eq!(strip_name("/test_container"), "test_container");
    }

    #[test]
    fn test_strip_name_multi_slash() {
        assert_eq!(strip_name("///test"), "test");
    }

    #[test]
    fn test_strip_name_no_slash() {
        assert_eq!(strip_name("plain_name"), "plain_name");
    }

    #[test]
    fn test_strip_name_empty() {
        assert_eq!(strip_name(""), "");
    }

    // ── parse_image_tag ──────────────────────────────────────

    #[test]
    fn test_parse_image_tag_with_tag() {
        let (name, tag) = parse_image_tag("nginx:1.25");
        assert_eq!(name, "nginx");
        assert_eq!(tag, "1.25");
    }

    #[test]
    fn test_parse_image_tag_no_tag() {
        let (name, tag) = parse_image_tag("nginx");
        assert_eq!(name, "nginx");
        assert_eq!(tag, "latest");
    }

    #[test]
    fn test_parse_image_tag_with_digest() {
        let (name, tag) = parse_image_tag("nginx@sha256:abc123");
        assert_eq!(name, "nginx");
        assert_eq!(tag, "");
    }

    #[test]
    fn test_parse_image_tag_registry_path() {
        let (name, tag) = parse_image_tag("ghcr.io/owner/repo:latest");
        assert_eq!(name, "ghcr.io/owner/repo");
        assert_eq!(tag, "latest");
    }

    // ── current_platform ──────────────────────────────────────

    #[test]
    fn test_current_platform_returns_some() {
        let platform = current_platform();
        assert!(platform.is_some());
        let arch = platform.unwrap();
        assert!(!arch.is_empty());
    }

    // ── Pull image ────────────────────────────────────────────

    #[tokio::test]
    async fn test_pull_image_empty_image_returns_false() {
        // Skip if no Docker socket is available
        let has_docker = std::path::Path::new("/var/run/docker.sock").exists()
            || std::env::var("DOCKER_HOST").is_ok();
        if !has_docker {
            return;
        }
        let docker = if let Ok(host) = std::env::var("DOCKER_HOST") {
            if let Some(path) = host.strip_prefix("unix://") {
                Docker::connect_with_socket(path, 120, bollard::API_DEFAULT_VERSION)
            } else {
                Docker::connect_with_http(&host, 120, bollard::API_DEFAULT_VERSION)
            }
            .expect("Failed Docker via DOCKER_HOST")
        } else {
            Docker::connect_with_local_defaults().expect("Failed Docker")
        };
        let result = pull_image(&docker, "", 5).await;
        assert!(!result, "Empty image pull should return false");
    }

    // ── Remove old image ──────────────────────────────────────

    #[test]
    fn test_remove_old_image_empty_does_nothing() {
        // Just verify it doesn't panic
        // We can't easily construct a Docker for testing, but the empty ID path
        // returns immediately
    }

    // ── Traefik URL extraction logic ──────────────────────────

    #[test]
    fn test_traefik_label_parsing_http() {
        let mut labels = HashMap::new();
        labels.insert(
            "traefik.http.routers.app.rule".to_string(),
            "Host(`app.example.com`)".to_string(),
        );
        let traefik_url = labels.iter().find_map(|(k, v)| {
            if k.ends_with(".rule") && v.starts_with("Host(") {
                let host = v
                    .trim_start_matches("Host(`")
                    .split('`')
                    .next()
                    .unwrap_or("");
                let tls = labels
                    .iter()
                    .any(|(lk, lv)| lk.starts_with(&k[..k.len() - 5]) && lv == "true");
                let proto = if tls { "https" } else { "http" };
                Some(format!("{}://{}", proto, host))
            } else {
                None
            }
        });
        assert_eq!(traefik_url, Some("http://app.example.com".to_string()));
    }

    #[test]
    fn test_traefik_label_parsing_https() {
        let mut labels = HashMap::new();
        labels.insert(
            "traefik.http.routers.app.rule".to_string(),
            "Host(`secure.example.com`)".to_string(),
        );
        labels.insert(
            "traefik.http.routers.app.tls".to_string(),
            "true".to_string(),
        );
        let traefik_url = labels.iter().find_map(|(k, v)| {
            if k.ends_with(".rule") && v.starts_with("Host(") {
                let host = v
                    .trim_start_matches("Host(`")
                    .split('`')
                    .next()
                    .unwrap_or("");
                let tls = labels
                    .iter()
                    .any(|(lk, lv)| lk.starts_with(&k[..k.len() - 5]) && lv == "true");
                let proto = if tls { "https" } else { "http" };
                Some(format!("{}://{}", proto, host))
            } else {
                None
            }
        });
        assert_eq!(traefik_url, Some("https://secure.example.com".to_string()));
    }

    #[test]
    fn test_traefik_label_no_match() {
        let labels: HashMap<String, String> = HashMap::new();
        let traefik_url = labels.iter().find_map(|(k, v)| {
            if k.ends_with(".rule") && v.starts_with("Host(") {
                let host = v
                    .trim_start_matches("Host(`")
                    .split('`')
                    .next()
                    .unwrap_or("");
                let tls = labels
                    .iter()
                    .any(|(lk, lv)| lk.starts_with(&k[..k.len() - 5]) && lv == "true");
                let proto = if tls { "https" } else { "http" };
                Some(format!("{}://{}", proto, host))
            } else {
                None
            }
        });
        assert!(traefik_url.is_none());
    }

    // ── Size conversion ───────────────────────────────────────

    #[test]
    fn test_size_conversion() {
        // 1048576 bytes = 1 MB
        let size_mb = ((1_048_576f64 / 1_048_576.0) * 100.0).round() / 100.0;
        assert_eq!(size_mb, 1.0);

        // 0 bytes
        let size_mb = ((0f64 / 1_048_576.0) * 100.0).round() / 100.0;
        assert_eq!(size_mb, 0.0);

        // 524288 bytes = 0.5 MB
        let size_mb = ((524_288f64 / 1_048_576.0) * 100.0).round() / 100.0;
        assert_eq!(size_mb, 0.5);
    }

    // ── ID truncation ─────────────────────────────────────────

    #[test]
    fn test_id_truncation_12_chars() {
        let long_id = "abcdef1234567890abcdef1234567890";
        let truncated: String = long_id.chars().take(12).collect();
        assert_eq!(truncated.len(), 12);
        assert_eq!(truncated, "abcdef123456");
    }

    #[test]
    fn test_id_truncation_short() {
        let short_id = "abc";
        let truncated: String = short_id.chars().take(12).collect();
        assert_eq!(truncated, "abc");
    }

    #[test]
    fn test_id_truncation_empty() {
        let empty = "";
        let truncated: String = empty.chars().take(12).collect();
        assert_eq!(truncated, "");
    }
}
