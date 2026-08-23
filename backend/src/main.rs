use std::sync::Arc;

use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use dockpot::auth::{self, AppState, JwtValidator};
use dockpot::config::Config;
use dockpot::db::Database;
use dockpot::embed::serve_embedded;
use dockpot::routes;

fn is_public_path(path: &str) -> bool {
    path == "/"
        || path == "/health"
        || path.starts_with("/auth/")
        || path.starts_with("/assets/")
        || path.ends_with(".html")
        || path.ends_with(".js")
        || path.ends_with(".css")
        || path.ends_with(".png")
        || path.ends_with(".ico")
        || path.ends_with(".svg")
        || path.ends_with(".woff2")
        || path.ends_with(".woff")
        || path.ends_with(".ttf")
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = Config::load();

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

    // ───── Connectivity verification ─────
    let check_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    match check_client
        .get(&format!("{}/.well-known/openid-configuration", config.oidc_issuer_url.trim_end_matches('/')))
        .send()
        .await
    {
        Ok(_) => tracing::info!("✅ OIDC provider reachable"),
        Err(e) => tracing::warn!("⚠️  OIDC provider not reachable: {} (will retry)", e),
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

    // ───── OIDC (mandatory) ─────
    let oidc_metadata = match auth::discover_oidc(&config).await {
        Ok(m) => {
            tracing::info!("✅ OIDC discovery: {}", m.issuer);
            m
        }
        Err(e) => {
            tracing::error!("❌ OIDC discovery failed: {}", e);
            std::process::exit(1);
        }
    };

    // ───── JWKS ─────
    let jwt_validator = JwtValidator::new(&config.oidc_issuer_url, &config.oidc_client_id);
    if let Err(e) = jwt_validator.fetch_jwks(&oidc_metadata.jwks_uri).await {
        tracing::error!("❌ JWKS fetch failed: {}", e);
        std::process::exit(1);
    }
    let jwt_validator = Arc::new(jwt_validator);

    // ───── App State ─────
    let (event_tx, _) = tokio::sync::broadcast::channel(auth::SSE_CHANNEL_CAPACITY);

    let app_state = Arc::new(AppState {
        config: config.clone(),
        db: db.clone(),
        oidc_metadata: Some(oidc_metadata),
        jwt_validator: jwt_validator.clone(),
        oidc_states: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        event_tx: event_tx.clone(),
    });

    // ───── Sync Scheduler ─────
    let db_for_sync = db.clone();
    tokio::spawn(async move {
        sync_scheduler_loop(db_for_sync).await;
    });

    // ───── Router ─────
    let state_for_middleware = app_state.clone();
    let app = routes::api_routes()
        .layer(CorsLayer::permissive())
        .layer(axum::middleware::from_fn(
            move |mut req: axum::extract::Request, next: axum::middleware::Next| {
                let state = state_for_middleware.clone();
                async move {
                    let path = req.uri().path().to_string();
                    req.extensions_mut().insert(state);
                    if is_public_path(&path) {
                        return Ok(next.run(req).await);
                    }
                    dockpot::auth::require_auth(req, next).await
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
                    db.update_sync_status(&sync.stack_id, "synced", commit.as_deref()).await.ok();
                    tracing::debug!("Sync for '{}': {}", stack.name, msg);
                }
                Err(e) => {
                    tracing::warn!("Sync failed for '{}': {}", stack.name, e);
                    db.update_sync_status(&sync.stack_id, "conflict", None).await.ok();
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
    }
}