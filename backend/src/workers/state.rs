use bollard::{container::InspectContainerOptions, system::EventsOptions, Docker};
use futures::{pin_mut, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};

use crate::containers::fetch_containers;
use crate::db::Database;
use crate::models::strip_name;
use crate::state::{ContainerInfo, NotifEvent, StateEvent};

pub type CachedContainers = Arc<RwLock<Option<Vec<ContainerInfo>>>>;

pub async fn docker_list_running(docker: &Docker) -> Vec<(String, String, String, Option<String>)> {
    match docker
        .list_containers(Some(bollard::container::ListContainersOptions::<String> {
            all: false,
            ..Default::default()
        }))
        .await
    {
        Ok(list) => {
            // Resolve bare digest images (e.g. "sha256:...") to their real repo tags
            let mut resolved_images: HashMap<String, String> = HashMap::new();
            for c in &list {
                let image = c.image.as_deref().unwrap_or("");
                if image.starts_with("sha256:") {
                    if let Some(cid) = &c.id {
                        if let Ok(inspect) = docker
                            .inspect_container(cid, None::<InspectContainerOptions>)
                            .await
                        {
                            let real = inspect
                                .config
                                .as_ref()
                                .and_then(|cfg| cfg.image.as_deref())
                                .unwrap_or("");
                            resolved_images.insert(cid.clone(), real.to_string());
                        }
                    }
                }
            }
            list.iter()
                .filter_map(|c| {
                    let name = c
                        .names
                        .as_ref()
                        .and_then(|n| n.first())
                        .map(|n| strip_name(n))?;
                    let raw_image = c.image.as_deref()?;
                    let image = if raw_image.starts_with("sha256:") {
                        c.id.as_ref()
                            .and_then(|cid| resolved_images.get(cid))
                            .map(|s| s.as_str())
                            .unwrap_or(raw_image)
                            .to_string()
                    } else {
                        raw_image.to_string()
                    };
                    let id = c.id.as_deref()?.to_string();
                    let image_id = c.image_id.as_deref().map(|s| s.to_string());
                    Some((name, image, id, image_id))
                })
                .collect()
        }
        Err(_) => vec![],
    }
}

async fn refresh(
    docker: &Docker,
    db: &Database,
    tx: &broadcast::Sender<StateEvent>,
    cache: &CachedContainers,
    notif_tx: &broadcast::Sender<NotifEvent>,
    previous_states: &mut HashMap<String, String>,
) {
    let containers = fetch_containers(docker, db).await;
    *cache.write().await = Some(containers.clone());
    let _ = tx.send(StateEvent {
        containers: containers.clone(),
    });

    // Detect state changes and emit notifications
    let now = chrono::Utc::now().to_rfc3339();
    for c in &containers {
        let prev = previous_states
            .get(&c.name)
            .map(|s| s.as_str())
            .unwrap_or("");
        let curr = c.state.as_str();
        if !prev.is_empty() && prev != curr {
            let _ = notif_tx.send(NotifEvent {
                container: c.name.clone(),
                status: format!("{} → {}", prev, curr),
                timestamp: now.clone(),
            });
        }
    }
    for c in &containers {
        previous_states.insert(c.name.clone(), c.state.clone());
    }
    let current_names: std::collections::HashSet<String> =
        containers.iter().map(|c| c.name.clone()).collect();
    previous_states.retain(|k, _| current_names.contains(k));
}

pub async fn state_worker(
    docker: Docker,
    db: Database,
    tx: broadcast::Sender<StateEvent>,
    cached_containers: CachedContainers,
    notif_tx: broadcast::Sender<NotifEvent>,
) {
    let relevant_actions = [
        "start", "stop", "die", "kill", "pause", "unpause", "restart", "create", "destroy",
        "rename", "update",
    ];

    let mut previous_states: HashMap<String, String> = HashMap::new();
    let mut consecutive_errors: u32 = 0;
    const MAX_CONSECUTIVE_ERRORS: u32 = 5;

    refresh(
        &docker,
        &db,
        &tx,
        &cached_containers,
        &notif_tx,
        &mut previous_states,
    )
    .await;

    loop {
        // ── After too many consecutive failures, fall back to polling only ──
        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
            tracing::warn!(
                "Docker events unavailable after {MAX_CONSECUTIVE_ERRORS} attempts — switching to polling-only mode"
            );
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                refresh(
                    &docker,
                    &db,
                    &tx,
                    &cached_containers,
                    &notif_tx,
                    &mut previous_states,
                )
                .await;
            }
        }

        let options = EventsOptions::<String> {
            since: None,
            until: None,
            filters: HashMap::new(),
        };
        let stream = docker.events(Some(options));
        pin_mut!(stream);
        let mut fallback = tokio::time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                event = stream.next() => {
                    match event {
                        Some(Ok(evt)) => {
                            consecutive_errors = 0;
                            if evt.typ == Some(bollard::models::EventMessageTypeEnum::CONTAINER) {
                                if let Some(ref action) = evt.action {
                                    if relevant_actions.contains(&action.as_str()) {
                                        refresh(&docker, &db, &tx, &cached_containers, &notif_tx, &mut previous_states).await;
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => {
                            consecutive_errors += 1;
                            let backoff = Duration::from_secs(consecutive_errors.min(10) as u64);
                            tracing::warn!(
                                "Docker events stream error (attempt {}/{}): {} — retrying in {}s",
                                consecutive_errors, MAX_CONSECUTIVE_ERRORS, e, backoff.as_secs()
                            );
                            tokio::time::sleep(backoff).await;
                            break;
                        }
                        None => {
                            tracing::warn!("Docker events stream ended — reconnecting");
                            consecutive_errors = 0;
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            break;
                        }
                    }
                }
                _ = fallback.tick() => {
                    refresh(&docker, &db, &tx, &cached_containers, &notif_tx, &mut previous_states).await;
                }
            }
        }
    }
}
