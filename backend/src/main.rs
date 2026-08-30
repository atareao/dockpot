use std::sync::Arc;

use bollard::Docker;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use dockpot::auth;
use dockpot::auth::DevMode;
use dockpot::config::Config;
use dockpot::db::Database;
use dockpot::embed::serve_embedded;
use dockpot::routes;
use dockpot::state::{AppState, JwtValidator};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = Config::load();
    let dev_mode = config.is_dev_mode();

    // ───── Tracing ─────
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    if config.log_format == "json" {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .pretty()
                    .with_target(true)
                    .with_file(true)
                    .with_line_number(true),
            )
            .init();
    }

    tracing::info!("🚀 Dockpot starting...");

    if dev_mode {
        tracing::warn!("⚠️  Modo dev sin OIDC — autenticación desactivada");
    }

    // ───── Connectivity verification ─────
    if !dev_mode {
        let check_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        match check_client
            .get(format!(
                "{}/.well-known/openid-configuration",
                config
                    .oidc_issuer_url
                    .as_deref()
                    .unwrap()
                    .trim_end_matches('/')
            ))
            .send()
            .await
        {
            Ok(_) => tracing::info!("✅ OIDC provider reachable"),
            Err(e) => tracing::warn!("⚠️  OIDC provider not reachable: {} (will retry)", e),
        }
    }

    // ───── Data directory ─────
    if let Err(e) = tokio::fs::create_dir_all(&config.data_dir).await {
        tracing::warn!("Could not create data dir: {}", e);
    }

    // ───── Database ─────
    let db = match Database::open(&config.database_url, &config.stacks_dir).await {
        Ok(db) => {
            tracing::info!("📦 Database opened: {}", config.database_url.display());
            db
        }
        Err(e) => {
            tracing::error!("❌ Failed to open database: {}", e);
            std::process::exit(1);
        }
    };

    // ───── Docker client ─────
    // connect_with_local_defaults() reads DOCKER_HOST env var automatically
    let docker = Docker::connect_with_local_defaults()
        .expect("Failed to connect to Docker daemon (is the socket accessible?)");

    // Ping to confirm connectivity
    match docker.ping().await {
        Ok(_) => tracing::info!("🐳 Docker daemon reachable"),
        Err(e) => tracing::info!("🐳 Docker connected (ping: {e})"),
    }

    // ───── OIDC (optional — skipped in dev mode) ─────
    let oidc_metadata = if !dev_mode {
        match auth::discover_oidc(
            config
                .oidc_issuer_url
                .as_deref()
                .expect("OIDC_ISSUER_URL must be set in non-dev mode"),
        )
        .await
        {
            Ok(m) => {
                tracing::info!("✅ OIDC discovery: {}", m.issuer);
                Some(m)
            }
            Err(e) => {
                tracing::error!("❌ OIDC discovery failed: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    // ───── JWKS ─────
    let jwt_validator = if !dev_mode {
        let validator = JwtValidator::new(
            config
                .oidc_issuer_url
                .as_deref()
                .expect("OIDC_ISSUER_URL must be set in non-dev mode"),
            config
                .oidc_client_id
                .as_deref()
                .expect("OIDC_CLIENT_ID must be set in non-dev mode"),
        );
        if let Err(e) = validator.fetch_jwks().await {
            tracing::error!("❌ JWKS fetch failed: {}", e);
            std::process::exit(1);
        }
        tracing::info!("✅ JWKS loaded");
        validator
    } else {
        JwtValidator::new("", "")
    };

    // ───── Broadcast channels ─────
    let (tx, _) = tokio::sync::broadcast::channel(256);
    let (update_tx, _) = tokio::sync::broadcast::channel(256);
    let (notif_tx, _) = tokio::sync::broadcast::channel(256);

    // ───── App State ─────
    let app_state = AppState {
        docker: docker.clone(),
        config: config.clone(),
        db: db.clone(),
        tx,
        update_tx,
        notif_tx,
        oidc_states: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        oidc_metadata,
        jwt_validator,
        cached_containers: Arc::new(tokio::sync::RwLock::new(None)),
        dev_mode,
    };

    // ───── Sync Scheduler ─────
    let db_for_sync = db.clone();
    tokio::spawn(async move {
        sync_scheduler_loop(db_for_sync).await;
    });

    // ───── Backup Scheduler ─────
    let db_for_backup = db.clone();
    tokio::spawn(async move {
        backup_scheduler_loop(db_for_backup).await;
    });

    // ───── State worker ─────
    let docker_for_worker = docker.clone();
    let tx_for_worker = app_state.tx.clone();
    let notif_tx_for_worker = app_state.notif_tx.clone();
    let cached_for_worker = app_state.cached_containers.clone();
    let db_for_worker = db.clone();
    tokio::spawn(async move {
        dockpot::workers::state_worker(
            docker_for_worker,
            db_for_worker,
            tx_for_worker,
            cached_for_worker,
            notif_tx_for_worker,
        )
        .await;
    });

    // ───── Cleanup worker ─────
    let docker_for_cleanup = docker.clone();
    tokio::spawn(async move {
        dockpot::workers::cleanup_worker(docker_for_cleanup).await;
    });

    // ───── Router ─────
    let client_secret = config.oidc_client_secret.clone().unwrap_or_default();
    let app = routes::api_routes()
        .layer(CorsLayer::permissive())
        .layer(axum::middleware::from_fn(
            move |headers: axum::http::HeaderMap,
                  mut req: axum::extract::Request,
                  next: axum::middleware::Next| {
                let secret = client_secret.clone();
                async move {
                    req.extensions_mut().insert(secret);
                    if dev_mode {
                        req.extensions_mut().insert(DevMode);
                    }
                    auth::auth_middleware(headers, req, next).await
                }
            },
        ))
        .fallback(|req: axum::extract::Request| async move {
            let path = req.uri().path().to_string();
            serve_embedded(&path).await
        })
        .with_state(app_state);

    // ───── Bind ─────
    let addr = if config.host == "0.0.0.0" {
        format!("[::]:{}", config.port)
    } else {
        format!("{}:{}", config.host, config.port)
    };

    tracing::info!("🌐 Dockpot en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("ctrl_c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Periodic sync scheduler: pulls remote changes for all git_remote stacks
async fn sync_scheduler_loop(db: dockpot::db::Database) {
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    loop {
        let configs = match db.list_sync_configs().await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Sync scheduler: failed to list configs: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                continue;
            }
        };

        for sync in &configs {
            if sync.sync_type != "git_remote" || sync.remote_url.is_none() {
                continue;
            }

            let stack = match db.get_stack(&sync.stack_id).await {
                Ok(Some(s)) => s,
                _ => continue,
            };

            let repo_path = std::path::Path::new(&stack.path);
            let repo = match git2::Repository::open(repo_path) {
                Ok(r) => r,
                Err(_) => continue,
            };

            match dockpot::git::sync::pull(&repo, &sync.remote_branch) {
                Ok(msg) => {
                    let commit = dockpot::git::sync::head_commit(&repo).ok().flatten();
                    db.update_sync_status(&sync.stack_id, "synced", commit.as_deref())
                        .await
                        .ok();
                    tracing::debug!("Sync for '{}': {}", stack.name, msg);
                }
                Err(e) => {
                    tracing::warn!("Sync failed for '{}': {}", stack.name, e);
                    db.update_sync_status(&sync.stack_id, "conflict", None)
                        .await
                        .ok();
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
    }
}

/// Periodic backup scheduler: runs on cron schedule
async fn backup_scheduler_loop(db: dockpot::db::Database) {
    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    loop {
        let schedule = match db.get_backup_schedule().await {
            Ok(Some(s)) => s,
            _ => {
                tokio::time::sleep(std::time::Duration::from_secs(300)).await;
                continue;
            }
        };
        if !schedule.enabled {
            tokio::time::sleep(std::time::Duration::from_secs(300)).await;
            continue;
        }

        // Simple cron check: run once per day at the configured hour
        // Parse cron "0 3 * * *" => run at 3:00
        if let Some(last_run) = &schedule.last_run_at {
            if let Ok(last) = chrono::DateTime::parse_from_rfc3339(last_run) {
                let now = chrono::Utc::now();
                let elapsed = now - last.with_timezone(&chrono::Utc);
                if elapsed.num_hours() < 23 {
                    tokio::time::sleep(std::time::Duration::from_secs(300)).await;
                    continue;
                }
            }
        }

        tracing::info!("⏰ Running scheduled backup...");
        if let Err(e) = db.run_backup().await {
            tracing::error!("❌ Scheduled backup failed: {}", e);
            db.update_backup_status(&format!("error: {}", e)).await.ok();
        }

        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    }
}
