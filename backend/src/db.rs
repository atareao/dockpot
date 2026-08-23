use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::*;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
    stacks_dir: String,
}

impl Database {
    pub async fn open(path: &Path, stacks_dir: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create data directory")?;
        }
        tokio::fs::create_dir_all(stacks_dir)
            .await
            .context("Failed to create stacks directory")?;

        let conn_opts = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(conn_opts)
            .await
            .context("Failed to open SQLite database")?;

        // Tables
        let tables = include_str!("../migrations/20260823000001_initial.sql");
        sqlx::raw_sql(tables).execute(&pool).await?;
        let sync_table = include_str!("../migrations/20260823000002_sync.sql");
        sqlx::raw_sql(sync_table).execute(&pool).await?;
        let agents_table = include_str!("../migrations/20260823000003_agents.sql");
        sqlx::raw_sql(agents_table).execute(&pool).await?;
        let features_table = include_str!("../migrations/20260823000004_features.sql");
        sqlx::raw_sql(features_table).execute(&pool).await?;
        let backup_table = include_str!("../migrations/20260823000005_backup.sql");
        sqlx::raw_sql(backup_table).execute(&pool).await?;

        Ok(Self {
            pool,
            stacks_dir: stacks_dir.to_string_lossy().to_string(),
        })
    }

    // ───── Stacks CRUD ─────

    pub async fn list_stacks(&self) -> Result<Vec<Stack>> {
        let rows = sqlx::query_as::<_, StackRow>(
            "SELECT id, name, description, compose, status, path, created_at, updated_at \
             FROM stacks ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Stack::from).collect())
    }

    pub async fn get_stack(&self, id: &str) -> Result<Option<Stack>> {
        let row = sqlx::query_as::<_, StackRow>(
            "SELECT id, name, description, compose, status, path, created_at, updated_at \
             FROM stacks WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Stack::from))
    }

    pub async fn get_stack_by_name(&self, name: &str) -> Result<Option<Stack>> {
        let row = sqlx::query_as::<_, StackRow>(
            "SELECT id, name, description, compose, status, path, created_at, updated_at \
             FROM stacks WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Stack::from))
    }

    pub async fn create_stack(&self, name: &str, description: Option<&str>, compose: &str) -> Result<Stack> {
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();
        let stacks_path = Path::new(&self.stacks_dir).join(&id);

        tokio::fs::create_dir_all(&stacks_path).await?;
        let compose_path = stacks_path.join("compose.yaml");
        tokio::fs::write(&compose_path, compose).await?;

        let path_str = stacks_path.to_string_lossy().to_string();
        sqlx::query(
            "INSERT INTO stacks (id, name, description, compose, status, path, created_at, updated_at) \
             VALUES (?, ?, ?, ?, 'stopped', ?, ?, ?)",
        )
        .bind(&id).bind(name).bind(description).bind(compose)
        .bind(&path_str).bind(&now).bind(&now)
        .execute(&self.pool).await?;

        Ok(Stack {
            id, name: name.to_string(),
            description: description.map(|s| s.to_string()),
            compose: compose.to_string(),
            status: "stopped".into(), path: path_str,
            created_at: now.clone(), updated_at: now,
        })
    }

    pub async fn update_stack(&self, id: &str, name: &str, description: Option<&str>, compose: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let rows = sqlx::query(
            "UPDATE stacks SET name=?, description=?, compose=?, updated_at=? WHERE id=?",
        )
        .bind(name).bind(description).bind(compose).bind(&now).bind(id)
        .execute(&self.pool).await?;

        if let Ok(Some(stack)) = self.get_stack(id).await {
            let compose_path = Path::new(&stack.path).join("compose.yaml");
            let _ = tokio::fs::write(&compose_path, compose).await;
        }
        Ok(rows.rows_affected() > 0)
    }

    pub async fn update_status(&self, id: &str, status: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let rows = sqlx::query("UPDATE stacks SET status=?, updated_at=? WHERE id=?")
            .bind(status).bind(&now).bind(id)
            .execute(&self.pool).await?;
        Ok(rows.rows_affected() > 0)
    }

    pub async fn delete_stack(&self, id: &str) -> Result<bool> {
        if let Ok(Some(stack)) = self.get_stack(id).await {
            let _ = tokio::fs::remove_dir_all(&stack.path).await;
        }
        let rows = sqlx::query("DELETE FROM stacks WHERE id=?")
            .bind(id).execute(&self.pool).await?;
        Ok(rows.rows_affected() > 0)
    }

    // ───── Stack Sync ─────

    pub async fn get_sync_config(&self, stack_id: &str) -> Result<Option<StackSync>> {
        let row = sqlx::query_as::<_, StackSyncRow>(
            "SELECT stack_id, sync_type, remote_url, remote_branch, auth_token, last_commit, last_synced_at, status \
             FROM stack_sync WHERE stack_id = ?",
        )
        .bind(stack_id).fetch_optional(&self.pool).await?;
        Ok(row.map(StackSync::from))
    }

    pub async fn upsert_sync_config(&self, sync: &StackSync) -> Result<()> {
        sqlx::query(
            "INSERT INTO stack_sync (stack_id, sync_type, remote_url, remote_branch, auth_token, last_commit, last_synced_at, status) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(stack_id) DO UPDATE SET \
             sync_type=excluded.sync_type, remote_url=excluded.remote_url, \
             remote_branch=excluded.remote_branch, auth_token=excluded.auth_token, \
             last_commit=excluded.last_commit, last_synced_at=excluded.last_synced_at, status=excluded.status",
        )
        .bind(&sync.stack_id).bind(&sync.sync_type).bind(&sync.remote_url)
        .bind(&sync.remote_branch).bind(&sync.auth_token).bind(&sync.last_commit)
        .bind(&sync.last_synced_at).bind(&sync.status)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn update_sync_status(&self, stack_id: &str, status: &str, last_commit: Option<&str>) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE stack_sync SET status=?, last_commit=COALESCE(?, last_commit), last_synced_at=? WHERE stack_id=?",
        )
        .bind(status).bind(last_commit).bind(&now).bind(stack_id)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn list_sync_configs(&self) -> Result<Vec<StackSync>> {
        let rows = sqlx::query_as::<_, StackSyncRow>(
            "SELECT stack_id, sync_type, remote_url, remote_branch, auth_token, last_commit, last_synced_at, status \
             FROM stack_sync ORDER BY stack_id",
        ).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(StackSync::from).collect())
    }

    // ───── Env Files ─────

    pub async fn list_env_files(&self, stack_id: &str) -> Result<Vec<EnvFile>> {
        let rows = sqlx::query_as::<_, EnvFileRow>(
            "SELECT id, stack_id, filename, content, created_at, updated_at FROM env_files WHERE stack_id=? ORDER BY filename",
        )
        .bind(stack_id).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(EnvFile::from).collect())
    }

    pub async fn upsert_env_file(&self, stack_id: &str, filename: &str, content: &str) -> Result<EnvFile> {
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO env_files (id, stack_id, filename, content, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT(stack_id, filename) DO UPDATE SET content=excluded.content, updated_at=excluded.updated_at",
        )
        .bind(&id).bind(stack_id).bind(filename).bind(content).bind(&now).bind(&now)
        .execute(&self.pool).await?;

        // Write to disk too
        if let Ok(Some(stack)) = self.get_stack(stack_id).await {
            let env_path = Path::new(&stack.path).join(filename);
            let _ = tokio::fs::write(&env_path, content).await;
        }

        Ok(EnvFile {
            id, stack_id: stack_id.to_string(),
            filename: filename.to_string(), content: content.to_string(),
            created_at: now.clone(), updated_at: now,
        })
    }

    pub async fn delete_env_file(&self, stack_id: &str, filename: &str) -> Result<bool> {
        let rows = sqlx::query("DELETE FROM env_files WHERE stack_id=? AND filename=?")
            .bind(stack_id).bind(filename)
            .execute(&self.pool).await?;
        if let Ok(Some(stack)) = self.get_stack(stack_id).await {
            let env_path = Path::new(&stack.path).join(filename);
            let _ = tokio::fs::remove_file(&env_path).await;
        }
        Ok(rows.rows_affected() > 0)
    }

    // ───── Log History ─────

    pub async fn append_logs(&self, stack_id: &str, logs: &str, level: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        // Insert in chunks to avoid oversized rows
        for chunk in logs.as_bytes().chunks(4096) {
            let text = String::from_utf8_lossy(chunk);
            sqlx::query("INSERT INTO log_history (stack_id, content, level, created_at) VALUES (?, ?, ?, ?)")
                .bind(stack_id).bind(text.as_ref()).bind(level).bind(&now)
                .execute(&self.pool).await?;
        }
        Ok(())
    }

    pub async fn get_logs(&self, stack_id: &str, limit: i64, offset: i64) -> Result<Vec<(String, String, String)>> {
        let rows: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT content, level, created_at FROM log_history \
             WHERE stack_id=? ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(stack_id).bind(limit).bind(offset)
        .fetch_all(&self.pool).await?;
        Ok(rows)
    }

    // ───── Notifiers ─────

    pub async fn list_notifiers(&self) -> Result<Vec<Notifier>> {
        let rows = sqlx::query_as::<_, NotifierRow>(
            "SELECT id, name, notifier_type, config_json, enabled, created_at, updated_at FROM notifiers ORDER BY name",
        ).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(Notifier::from).collect())
    }

    pub async fn get_notifier(&self, id: &str) -> Result<Option<Notifier>> {
        let row = sqlx::query_as::<_, NotifierRow>(
            "SELECT id, name, notifier_type, config_json, enabled, created_at, updated_at FROM notifiers WHERE id=?",
        ).bind(id).fetch_optional(&self.pool).await?;
        Ok(row.map(Notifier::from))
    }

    pub async fn create_notifier(&self, notifier: &Notifier) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let config = serde_json::to_string(&notifier.config_json).unwrap_or_default();
        sqlx::query(
            "INSERT INTO notifiers (id, name, notifier_type, config_json, enabled, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&notifier.id).bind(&notifier.name).bind(&notifier.notifier_type)
        .bind(&config).bind(notifier.enabled as i32).bind(&now).bind(&now)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn update_notifier(&self, id: &str, notifier: &Notifier) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let config = serde_json::to_string(&notifier.config_json).unwrap_or_default();
        let rows = sqlx::query(
            "UPDATE notifiers SET name=?, notifier_type=?, config_json=?, enabled=?, updated_at=? WHERE id=?",
        )
        .bind(&notifier.name).bind(&notifier.notifier_type).bind(&config)
        .bind(notifier.enabled as i32).bind(&now).bind(id)
        .execute(&self.pool).await?;
        Ok(rows.rows_affected() > 0)
    }

    pub async fn delete_notifier(&self, id: &str) -> Result<bool> {
        let rows = sqlx::query("DELETE FROM notifiers WHERE id=?")
            .bind(id).execute(&self.pool).await?;
        Ok(rows.rows_affected() > 0)
    }

    pub async fn set_stack_notifiers(&self, stack_id: &str, notifier_ids: &[String]) -> Result<()> {
        sqlx::query("DELETE FROM stack_notifiers WHERE stack_id=?")
            .bind(stack_id).execute(&self.pool).await?;
        for nid in notifier_ids {
            sqlx::query("INSERT OR IGNORE INTO stack_notifiers (stack_id, notifier_id) VALUES (?, ?)")
                .bind(stack_id).bind(nid).execute(&self.pool).await?;
        }
        Ok(())
    }

    pub async fn get_stack_notifier_ids(&self, stack_id: &str) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT notifier_id FROM stack_notifiers WHERE stack_id=?",
        ).bind(stack_id).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    // ───── Stack Stats ─────

    pub async fn get_stats(&self, stack_id: &str) -> Result<Option<StackStats>> {
        let row: Option<(Option<String>, i64)> = sqlx::query_as(
            "SELECT last_started_at, total_running_seconds FROM stack_stats WHERE stack_id=?",
        )
        .bind(stack_id).fetch_optional(&self.pool).await?;
        Ok(row.map(|(started, secs)| StackStats {
            stack_id: stack_id.to_string(),
            last_started_at: started,
            total_running_seconds: secs,
        }))
    }

    pub async fn record_start(&self, stack_id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO stack_stats (stack_id, last_started_at, total_running_seconds) VALUES (?, ?, 0) \
             ON CONFLICT(stack_id) DO UPDATE SET last_started_at=excluded.last_started_at",
        )
        .bind(stack_id).bind(&now)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn record_stop(&self, stack_id: &str) -> Result<()> {
        // Calculate elapsed since last start and add to total
        if let Ok(Some(stats)) = self.get_stats(stack_id).await {
            if let Some(started) = stats.last_started_at {
                if let Ok(t) = chrono::DateTime::parse_from_rfc3339(&started) {
                    let elapsed = (Utc::now() - t.with_timezone(&Utc)).num_seconds().max(0);
                    sqlx::query("UPDATE stack_stats SET total_running_seconds = total_running_seconds + ?, last_started_at = NULL WHERE stack_id=?")
                        .bind(elapsed).bind(stack_id)
                        .execute(&self.pool).await?;
                }
            }
        }
        Ok(())
    }

    // ───── Dashboard ─────

    pub async fn get_dashboard_status(&self) -> Result<DashboardStatus> {
        use crate::models::{DashboardStatus, RecentActivity};
        let stacks = self.list_stacks().await?;
        let total = stacks.len() as u64;
        let running = stacks.iter().filter(|s| s.status == "running").count() as u64;
        let stopped = stacks.iter().filter(|s| s.status == "stopped").count() as u64;
        let error = stacks.iter().filter(|s| s.status == "error").count() as u64;

        // Docker info from CLI
        let docker_version = tokio::process::Command::new("docker")
            .args(["version", "--format", "{{.Server.Version}}"])
            .output().await.ok()
            .and_then(|o| {
                let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if v.is_empty() { None } else { Some(v) }
            });

        let docker_containers = tokio::process::Command::new("docker")
            .args(["ps", "-q"])
            .output().await.ok()
            .map(|o| {
                let out = String::from_utf8_lossy(&o.stdout);
                out.lines().filter(|l| !l.is_empty()).count() as u64
            }).unwrap_or(0);

        let docker_images = tokio::process::Command::new("docker")
            .args(["images", "-q"])
            .output().await.ok()
            .map(|o| {
                let out = String::from_utf8_lossy(&o.stdout);
                out.lines().filter(|l| !l.is_empty()).count() as u64
            }).unwrap_or(0);

        // Recent activity from log_history
        let recent: Vec<(String, String)> = sqlx::query_as(
            "SELECT lh.stack_id, lh.created_at FROM log_history lh \
             INNER JOIN (SELECT stack_id, MAX(id) as max_id FROM log_history GROUP BY stack_id) latest \
             ON lh.id = latest.max_id ORDER BY lh.created_at DESC LIMIT 5",
        )
        .fetch_all(&self.pool).await
        .unwrap_or_default();

        let mut recent_activity = Vec::new();
        for (sid, ts) in &recent {
            if let Ok(Some(s)) = self.get_stack(sid).await {
                recent_activity.push(RecentActivity {
                    stack_name: s.name,
                    action: format!("log entry at {}", ts),
                    timestamp: ts.clone(),
                });
            }
        }

        Ok(DashboardStatus {
            total_stacks: total,
            running_stacks: running,
            stopped_stacks: stopped,
            error_stacks: error,
            docker_version,
            docker_containers,
            docker_images,
            recent_activity,
        })
    }

    // ───── Settings ─────

    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        sqlx::query_scalar("SELECT value FROM settings WHERE key=?")
            .bind(key).fetch_optional(&self.pool).await
            .context("Failed to get setting")
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
            .bind(key).bind(value)
            .execute(&self.pool).await?;
        Ok(())
    }

    // ───── Agents ─────

    pub async fn list_agents(&self) -> Result<Vec<crate::models::Agent>> {
        let rows = sqlx::query_as::<_, AgentRow>(
            "SELECT id, name, agent_type, host, port, tls_enabled, ca_cert, client_cert, client_key, \
             description, enabled, created_at, updated_at FROM agents ORDER BY name",
        ).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn get_agent(&self, id: &str) -> Result<Option<Agent>> {
        let row = sqlx::query_as::<_, AgentRow>(
            "SELECT id, name, agent_type, host, port, tls_enabled, ca_cert, client_cert, client_key, \
             description, enabled, created_at, updated_at FROM agents WHERE id = ?",
        ).bind(id).fetch_optional(&self.pool).await?;
        Ok(row.map(|r| r.into()))
    }

    pub async fn create_agent(&self, agent: &Agent) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO agents (id, name, agent_type, host, port, tls_enabled, ca_cert, client_cert, client_key, \
             description, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&agent.id).bind(&agent.name).bind(&agent.agent_type).bind(&agent.host)
        .bind(agent.port as i64).bind(agent.tls_enabled as i32)
        .bind(&agent.ca_cert).bind(&agent.client_cert).bind(&agent.client_key)
        .bind(&agent.description).bind(agent.enabled as i32)
        .bind(&now).bind(&now)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn update_agent(&self, id: &str, agent: &Agent) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let rows = sqlx::query(
            "UPDATE agents SET name=?, host=?, port=?, tls_enabled=?, ca_cert=?, client_cert=?, client_key=?, \
             description=?, enabled=?, updated_at=? WHERE id=?",
        )
        .bind(&agent.name).bind(&agent.host).bind(agent.port as i64).bind(agent.tls_enabled as i32)
        .bind(&agent.ca_cert).bind(&agent.client_cert).bind(&agent.client_key)
        .bind(&agent.description).bind(agent.enabled as i32).bind(&now).bind(id)
        .execute(&self.pool).await?;
        Ok(rows.rows_affected() > 0)
    }

    pub async fn delete_agent(&self, id: &str) -> Result<bool> {
        let rows = sqlx::query("DELETE FROM agents WHERE id=?")
            .bind(id).execute(&self.pool).await?;
        Ok(rows.rows_affected() > 0)
    }

    // ───── Backup Schedule ─────

    pub async fn get_backup_schedule(&self) -> Result<Option<BackupSchedule>> {
        let row = sqlx::query_as::<_, BackupScheduleRow>(
            "SELECT id, enabled, cron_expression, retention_days, include_git, include_env, \
             last_run_at, last_status, created_at, updated_at FROM backup_schedules LIMIT 1",
        )
        .fetch_optional(&self.pool).await?;
        Ok(row.map(BackupSchedule::from))
    }

    pub async fn upsert_backup_schedule(&self, s: &BackupSchedule) -> Result<()> {
        let existing = self.get_backup_schedule().await?;
        let id = existing.as_ref().map(|e| e.id.clone()).unwrap_or_else(|| s.id.clone());
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO backup_schedules (id, enabled, cron_expression, retention_days, include_git, include_env, \
             last_run_at, last_status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET enabled=excluded.enabled, cron_expression=excluded.cron_expression, \
             retention_days=excluded.retention_days, include_git=excluded.include_git, include_env=excluded.include_env, \
             updated_at=excluded.updated_at",
        )
        .bind(&id).bind(s.enabled as i32).bind(&s.cron_expression).bind(s.retention_days)
        .bind(s.include_git as i32).bind(s.include_env as i32)
        .bind(&s.last_run_at).bind(&s.last_status).bind(&now).bind(&now)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn update_backup_status(&self, status: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE backup_schedules SET last_run_at=?, last_status=?")
            .bind(&now).bind(status)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn run_backup(&self) -> Result<String, String> {
        let schedule = self.get_backup_schedule().await
            .map_err(|e| format!("DB error: {}", e))?;
        let backup_dir = std::path::Path::new("data").join("backups");
        tokio::fs::create_dir_all(&backup_dir).await
            .map_err(|e| format!("Failed to create backup dir: {}", e))?;
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("dockpot-backup-{}.zip", timestamp);
        let output_path = backup_dir.join(&filename);
        let tmp_dir = std::env::temp_dir().join(format!("dockpot-backup-{}", timestamp));
        tokio::fs::create_dir_all(&tmp_dir).await
            .map_err(|e| format!("Failed to create temp dir: {}", e))?;

        let stacks = self.list_stacks().await.map_err(|e| format!("List stacks: {}", e))?;
        for stack in &stacks {
            let src = std::path::Path::new(&stack.path);
            let dst = tmp_dir.join(&stack.name);
            if src.exists() {
                let _ = std::process::Command::new("cp")
                    .args(["-r", src.to_str().unwrap_or(""), dst.to_str().unwrap_or("")])
                    .output();
            }
        }
        let db_src = std::path::Path::new("data/dockpot.db");
        if db_src.exists() {
            let db_dst = tmp_dir.join("dockpot.db");
            let _ = tokio::fs::copy(db_src, &db_dst).await;
        }
        let _ = std::process::Command::new("zip")
            .args(["-r", output_path.to_str().unwrap_or(""), "."])
            .current_dir(&tmp_dir).output();
        let _ = tokio::fs::remove_dir_all(&tmp_dir).await;

        if let Some(ref sched) = schedule {
            let cutoff = chrono::Utc::now() - chrono::Duration::days(sched.retention_days);
            if tokio::fs::try_exists(&backup_dir).await.unwrap_or(false) {
                let mut dir = tokio::fs::read_dir(&backup_dir).await.unwrap();
                while let Ok(Some(entry)) = dir.next_entry().await {
                    if let Ok(meta) = entry.metadata().await {
                        if meta.is_file() {
                            if let Ok(modified) = meta.modified() {
                                let modified: chrono::DateTime<chrono::Utc> = modified.into();
                                if modified < cutoff {
                                    tokio::fs::remove_file(entry.path()).await.ok();
                                }
                            }
                        }
                    }
                }
            }
        }
        self.update_backup_status("ok").await.map_err(|e| format!("Status update: {}", e))?;
        tracing::info!("📦 Backup created: {}", output_path.display());
        self.append_logs("system", &format!("📦 Backup created: {}\n", filename), "info").await.ok();
        Ok(output_path.to_string_lossy().to_string())
    }
}