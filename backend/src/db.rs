use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;
use deadpool_sqlite::{Config as PoolConfig, Pool, Runtime};
use uuid::Uuid;

use crate::models::*;

#[derive(Clone)]
pub struct Database {
    pool: Pool,
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

        let config = PoolConfig::new(path);
        let pool = config
            .create_pool(Runtime::Tokio1)
            .context("Failed to create deadpool-sqlite pool")?;

        // Run migrations
        {
            let obj = pool.get().await?;
            let conn = obj.lock().unwrap();
            conn.execute_batch("PRAGMA journal_mode=WAL;")?;
            conn.execute_batch("PRAGMA foreign_keys=ON;")?;
            let tables = include_str!("../migrations/20260823000001_initial.sql");
            conn.execute_batch(tables)?;
            let sync_table = include_str!("../migrations/20260823000002_sync.sql");
            conn.execute_batch(sync_table)?;
            let features_table = include_str!("../migrations/20260823000004_features.sql");
            conn.execute_batch(features_table)?;
            let backup_table = include_str!("../migrations/20260823000005_backup.sql");
            conn.execute_batch(backup_table)?;
        }

        Ok(Self {
            pool,
            stacks_dir: stacks_dir.to_string_lossy().to_string(),
        })
    }

    // ───── Stacks CRUD ─────

    pub async fn list_stacks(&self) -> Result<Vec<Stack>> {
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, compose, status, path, created_at, updated_at \
             FROM stacks ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Stack {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                compose: row.get(3)?,
                status: row.get(4)?,
                path: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        let mut stacks = Vec::new();
        for r in rows {
            stacks.push(r?);
        }
        Ok(stacks)
    }

    pub async fn get_stack(&self, id: &str) -> Result<Option<Stack>> {
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, compose, status, path, created_at, updated_at \
             FROM stacks WHERE id = ?",
        )?;
        let mut rows = stmt.query_map([id], |row| {
            Ok(Stack {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                compose: row.get(3)?,
                status: row.get(4)?,
                path: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        match rows.next() {
            Some(Ok(stack)) => Ok(Some(stack)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    pub async fn get_stack_by_name(&self, name: &str) -> Result<Option<Stack>> {
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, compose, status, path, created_at, updated_at \
             FROM stacks WHERE name = ?",
        )?;
        let mut rows = stmt.query_map([name], |row| {
            Ok(Stack {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                compose: row.get(3)?,
                status: row.get(4)?,
                path: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        match rows.next() {
            Some(Ok(stack)) => Ok(Some(stack)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    pub async fn create_stack(
        &self,
        name: &str,
        description: Option<&str>,
        compose: &str,
    ) -> Result<Stack> {
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();
        let stacks_path = Path::new(&self.stacks_dir).join(&id);

        tokio::fs::create_dir_all(&stacks_path).await?;
        let compose_path = stacks_path.join("compose.yaml");
        tokio::fs::write(&compose_path, compose).await?;

        let path_str = stacks_path.to_string_lossy().to_string();
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        conn.execute(
            "INSERT INTO stacks (id, name, description, compose, status, path, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, 'stopped', ?5, ?6, ?7)",
            rusqlite::params![id, name, description, compose, path_str, now, now],
        )?;

        Ok(Stack {
            id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            compose: compose.to_string(),
            status: "stopped".into(),
            path: path_str,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub async fn update_stack(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        compose: &str,
    ) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let rows = conn.execute(
            "UPDATE stacks SET name=?1, description=?2, compose=?3, updated_at=?4 WHERE id=?5",
            rusqlite::params![name, description, compose, now, id],
        )?;

        if let Ok(Some(stack)) = self.get_stack(id).await {
            let compose_path = Path::new(&stack.path).join("compose.yaml");
            let _ = tokio::fs::write(&compose_path, compose).await;
        }
        Ok(rows > 0)
    }

    pub async fn update_status(&self, id: &str, status: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let rows = conn.execute(
            "UPDATE stacks SET status=?1, updated_at=?2 WHERE id=?3",
            rusqlite::params![status, now, id],
        )?;
        Ok(rows > 0)
    }

    pub async fn delete_stack(&self, id: &str) -> Result<bool> {
        if let Ok(Some(stack)) = self.get_stack(id).await {
            let _ = tokio::fs::remove_dir_all(&stack.path).await;
        }
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let rows = conn.execute("DELETE FROM stacks WHERE id=?", [id])?;
        Ok(rows > 0)
    }

    // ───── Stack Sync ─────

    pub async fn get_sync_config(&self, stack_id: &str) -> Result<Option<StackSync>> {
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT stack_id, sync_type, remote_url, remote_branch, auth_token, last_commit, \
             last_synced_at, status FROM stack_sync WHERE stack_id = ?",
        )?;
        let mut rows = stmt.query_map([stack_id], |row| {
            Ok(StackSync {
                stack_id: row.get(0)?,
                sync_type: row.get(1)?,
                remote_url: row.get(2)?,
                remote_branch: row.get(3)?,
                auth_token: row.get(4)?,
                last_commit: row.get(5)?,
                last_synced_at: row.get(6)?,
                status: row.get(7)?,
            })
        })?;
        match rows.next() {
            Some(Ok(sync)) => Ok(Some(sync)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    pub async fn upsert_sync_config(&self, sync: &StackSync) -> Result<()> {
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        conn.execute(
            "INSERT INTO stack_sync (stack_id, sync_type, remote_url, remote_branch, auth_token, \
             last_commit, last_synced_at, status) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
             ON CONFLICT(stack_id) DO UPDATE SET \
             sync_type=excluded.sync_type, remote_url=excluded.remote_url, \
             remote_branch=excluded.remote_branch, auth_token=excluded.auth_token, \
             last_commit=excluded.last_commit, last_synced_at=excluded.last_synced_at, \
             status=excluded.status",
            rusqlite::params![
                sync.stack_id,
                sync.sync_type,
                sync.remote_url,
                sync.remote_branch,
                sync.auth_token,
                sync.last_commit,
                sync.last_synced_at,
                sync.status,
            ],
        )?;
        Ok(())
    }

    pub async fn update_sync_status(
        &self,
        stack_id: &str,
        status: &str,
        last_commit: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        conn.execute(
            "UPDATE stack_sync SET status=?1, last_commit=COALESCE(?2, last_commit), \
             last_synced_at=?3 WHERE stack_id=?4",
            rusqlite::params![status, last_commit, now, stack_id],
        )?;
        Ok(())
    }

    pub async fn list_sync_configs(&self) -> Result<Vec<StackSync>> {
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT stack_id, sync_type, remote_url, remote_branch, auth_token, last_commit, \
             last_synced_at, status FROM stack_sync ORDER BY stack_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(StackSync {
                stack_id: row.get(0)?,
                sync_type: row.get(1)?,
                remote_url: row.get(2)?,
                remote_branch: row.get(3)?,
                auth_token: row.get(4)?,
                last_commit: row.get(5)?,
                last_synced_at: row.get(6)?,
                status: row.get(7)?,
            })
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    // ───── Env Files ─────

    pub async fn list_env_files(&self, stack_id: &str) -> Result<Vec<EnvFile>> {
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, stack_id, filename, content, created_at, updated_at \
             FROM env_files WHERE stack_id=? ORDER BY filename",
        )?;
        let rows = stmt.query_map([stack_id], |row| {
            Ok(EnvFile {
                id: row.get(0)?,
                stack_id: row.get(1)?,
                filename: row.get(2)?,
                content: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub async fn upsert_env_file(
        &self,
        stack_id: &str,
        filename: &str,
        content: &str,
    ) -> Result<EnvFile> {
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        conn.execute(
            "INSERT INTO env_files (id, stack_id, filename, content, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(stack_id, filename) DO UPDATE SET \
             content=excluded.content, updated_at=excluded.updated_at",
            rusqlite::params![id, stack_id, filename, content, now, now],
        )?;

        // Write to disk too
        if let Ok(Some(stack)) = self.get_stack(stack_id).await {
            let env_path = Path::new(&stack.path).join(filename);
            let _ = tokio::fs::write(&env_path, content).await;
        }

        Ok(EnvFile {
            id,
            stack_id: stack_id.to_string(),
            filename: filename.to_string(),
            content: content.to_string(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub async fn delete_env_file(&self, stack_id: &str, filename: &str) -> Result<bool> {
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let rows = conn.execute(
            "DELETE FROM env_files WHERE stack_id=?1 AND filename=?2",
            rusqlite::params![stack_id, filename],
        )?;
        if let Ok(Some(stack)) = self.get_stack(stack_id).await {
            let env_path = Path::new(&stack.path).join(filename);
            let _ = tokio::fs::remove_file(&env_path).await;
        }
        Ok(rows > 0)
    }

    // ───── Log History ─────

    pub async fn append_logs(&self, stack_id: &str, logs: &str, level: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        // Insert in chunks to avoid oversized rows
        for chunk in logs.as_bytes().chunks(4096) {
            let text = String::from_utf8_lossy(chunk);
            conn.execute(
                "INSERT INTO log_history (stack_id, content, level, created_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![stack_id, text.as_ref(), level, now],
            )?;
        }
        Ok(())
    }

    pub async fn get_logs(
        &self,
        stack_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<(String, String, String)>> {
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT content, level, created_at FROM log_history \
             WHERE stack_id=? ORDER BY id DESC LIMIT ? OFFSET ?",
        )?;
        let rows = stmt.query_map(rusqlite::params![stack_id, limit, offset], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    // ───── Notifiers ─────

    pub async fn list_notifiers(&self) -> Result<Vec<Notifier>> {
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, notifier_type, config_json, enabled, created_at, updated_at \
             FROM notifiers ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            let config_json: String = row.get(3)?;
            Ok(Notifier {
                id: row.get(0)?,
                name: row.get(1)?,
                notifier_type: row.get(2)?,
                config_json: serde_json::from_str(&config_json).unwrap_or_default(),
                enabled: row.get::<_, i32>(4)? != 0,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub async fn get_notifier(&self, id: &str) -> Result<Option<Notifier>> {
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, notifier_type, config_json, enabled, created_at, updated_at \
             FROM notifiers WHERE id=?",
        )?;
        let mut rows = stmt.query_map([id], |row| {
            let config_json: String = row.get(3)?;
            Ok(Notifier {
                id: row.get(0)?,
                name: row.get(1)?,
                notifier_type: row.get(2)?,
                config_json: serde_json::from_str(&config_json).unwrap_or_default(),
                enabled: row.get::<_, i32>(4)? != 0,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        match rows.next() {
            Some(Ok(n)) => Ok(Some(n)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    pub async fn create_notifier(&self, notifier: &Notifier) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let config = serde_json::to_string(&notifier.config_json).unwrap_or_default();
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        conn.execute(
            "INSERT INTO notifiers (id, name, notifier_type, config_json, enabled, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                notifier.id,
                notifier.name,
                notifier.notifier_type,
                config,
                notifier.enabled as i32,
                now,
                now,
            ],
        )?;
        Ok(())
    }

    pub async fn update_notifier(&self, id: &str, notifier: &Notifier) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let config = serde_json::to_string(&notifier.config_json).unwrap_or_default();
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let rows = conn.execute(
            "UPDATE notifiers SET name=?1, notifier_type=?2, config_json=?3, enabled=?4, \
             updated_at=?5 WHERE id=?6",
            rusqlite::params![
                notifier.name,
                notifier.notifier_type,
                config,
                notifier.enabled as i32,
                now,
                id,
            ],
        )?;
        Ok(rows > 0)
    }

    pub async fn delete_notifier(&self, id: &str) -> Result<bool> {
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let rows = conn.execute("DELETE FROM notifiers WHERE id=?", [id])?;
        Ok(rows > 0)
    }

    pub async fn set_stack_notifiers(&self, stack_id: &str, notifier_ids: &[String]) -> Result<()> {
        let obj = self.pool.get().await?;
        let mut conn = obj.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM stack_notifiers WHERE stack_id=?", [stack_id])?;
        for nid in notifier_ids {
            tx.execute(
                "INSERT OR IGNORE INTO stack_notifiers (stack_id, notifier_id) VALUES (?1, ?2)",
                rusqlite::params![stack_id, nid],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub async fn get_stack_notifier_ids(&self, stack_id: &str) -> Result<Vec<String>> {
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let mut stmt = conn.prepare("SELECT notifier_id FROM stack_notifiers WHERE stack_id=?")?;
        let rows = stmt.query_map([stack_id], |row| row.get::<_, String>(0))?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    // ───── Stack Stats ─────

    pub async fn get_stats(&self, stack_id: &str) -> Result<Option<StackStats>> {
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT last_started_at, total_running_seconds FROM stack_stats WHERE stack_id=?",
        )?;
        let mut rows = stmt.query_map([stack_id], |row| {
            Ok(StackStats {
                stack_id: stack_id.to_string(),
                last_started_at: row.get(0)?,
                total_running_seconds: row.get(1)?,
            })
        })?;
        match rows.next() {
            Some(Ok(stats)) => Ok(Some(stats)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    pub async fn record_start(&self, stack_id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        conn.execute(
            "INSERT INTO stack_stats (stack_id, last_started_at, total_running_seconds) \
             VALUES (?1, ?2, 0) \
             ON CONFLICT(stack_id) DO UPDATE SET last_started_at=excluded.last_started_at",
            rusqlite::params![stack_id, now],
        )?;
        Ok(())
    }

    pub async fn record_stop(&self, stack_id: &str) -> Result<()> {
        // Calculate elapsed since last start and add to total
        if let Ok(Some(stats)) = self.get_stats(stack_id).await {
            if let Some(started) = stats.last_started_at {
                if let Ok(t) = chrono::DateTime::parse_from_rfc3339(&started) {
                    let elapsed = (Utc::now() - t.with_timezone(&Utc)).num_seconds().max(0);
                    let obj = self.pool.get().await?;
                    let conn = obj.lock().unwrap();
                    conn.execute(
                        "UPDATE stack_stats SET total_running_seconds = total_running_seconds + ?1, \
                         last_started_at = NULL WHERE stack_id=?2",
                        rusqlite::params![elapsed, stack_id],
                    )?;
                }
            }
        }
        Ok(())
    }

    // ───── Dashboard ─────

    pub async fn get_dashboard_status(&self) -> Result<DashboardStatus> {
        let stacks = self.list_stacks().await?;
        let total = stacks.len() as u64;
        let running = stacks.iter().filter(|s| s.status == "running").count() as u64;
        let stopped = stacks.iter().filter(|s| s.status == "stopped").count() as u64;
        let error = stacks.iter().filter(|s| s.status == "error").count() as u64;

        // Docker info from CLI
        let docker_version = tokio::process::Command::new("docker")
            .args(["version", "--format", "{{.Server.Version}}"])
            .output()
            .await
            .ok()
            .and_then(|o| {
                let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            });

        let docker_containers = tokio::process::Command::new("docker")
            .args(["ps", "-q"])
            .output()
            .await
            .ok()
            .map(|o| {
                let out = String::from_utf8_lossy(&o.stdout);
                out.lines().filter(|l| !l.is_empty()).count() as u64
            })
            .unwrap_or(0);

        let docker_images = tokio::process::Command::new("docker")
            .args(["images", "-q"])
            .output()
            .await
            .ok()
            .map(|o| {
                let out = String::from_utf8_lossy(&o.stdout);
                out.lines().filter(|l| !l.is_empty()).count() as u64
            })
            .unwrap_or(0);

        // Recent activity from log_history
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT lh.stack_id, lh.created_at FROM log_history lh \
             INNER JOIN (SELECT stack_id, MAX(id) as max_id FROM log_history GROUP BY stack_id) latest \
             ON lh.id = latest.max_id ORDER BY lh.created_at DESC LIMIT 5",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut recent: Vec<(String, String)> = Vec::new();
        for r in rows {
            recent.push(r.unwrap_or_default());
        }

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
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key=?")?;
        let mut rows = stmt.query_map([key], |row| row.get::<_, String>(0))?;
        match rows.next() {
            Some(Ok(val)) => Ok(Some(val)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) \
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    // ───── Backup Schedule ─────

    pub async fn get_backup_schedule(&self) -> Result<Option<BackupSchedule>> {
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, enabled, cron_expression, retention_days, include_git, include_env, \
             last_run_at, last_status, created_at, updated_at FROM backup_schedules LIMIT 1",
        )?;
        let mut rows = stmt.query_map([], |row| {
            Ok(BackupSchedule {
                id: row.get(0)?,
                enabled: row.get::<_, i32>(1)? != 0,
                cron_expression: row.get(2)?,
                retention_days: row.get(3)?,
                include_git: row.get::<_, i32>(4)? != 0,
                include_env: row.get::<_, i32>(5)? != 0,
                last_run_at: row.get(6)?,
                last_status: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        match rows.next() {
            Some(Ok(sched)) => Ok(Some(sched)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    pub async fn upsert_backup_schedule(&self, s: &BackupSchedule) -> Result<()> {
        let existing = self.get_backup_schedule().await?;
        let id = existing
            .as_ref()
            .map(|e| e.id.clone())
            .unwrap_or_else(|| s.id.clone());
        let now = chrono::Utc::now().to_rfc3339();
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        conn.execute(
            "INSERT INTO backup_schedules \
             (id, enabled, cron_expression, retention_days, include_git, include_env, \
              last_run_at, last_status, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
             ON CONFLICT(id) DO UPDATE SET \
             enabled=excluded.enabled, cron_expression=excluded.cron_expression, \
             retention_days=excluded.retention_days, include_git=excluded.include_git, \
             include_env=excluded.include_env, updated_at=excluded.updated_at",
            rusqlite::params![
                id,
                s.enabled as i32,
                s.cron_expression,
                s.retention_days,
                s.include_git as i32,
                s.include_env as i32,
                s.last_run_at,
                s.last_status,
                now,
                now,
            ],
        )?;
        Ok(())
    }

    pub async fn update_backup_status(&self, status: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let obj = self.pool.get().await?;
        let conn = obj.lock().unwrap();
        conn.execute(
            "UPDATE backup_schedules SET last_run_at=?1, last_status=?2",
            rusqlite::params![now, status],
        )?;
        Ok(())
    }

    pub async fn run_backup(&self) -> Result<String, String> {
        let schedule = self
            .get_backup_schedule()
            .await
            .map_err(|e| format!("DB error: {}", e))?;
        let backup_dir = std::path::Path::new("data").join("backups");
        tokio::fs::create_dir_all(&backup_dir)
            .await
            .map_err(|e| format!("Failed to create backup dir: {}", e))?;
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("dockpot-backup-{}.zip", timestamp);
        let output_path = backup_dir.join(&filename);
        let tmp_dir = std::env::temp_dir().join(format!("dockpot-backup-{}", timestamp));
        tokio::fs::create_dir_all(&tmp_dir)
            .await
            .map_err(|e| format!("Failed to create temp dir: {}", e))?;

        let stacks = self
            .list_stacks()
            .await
            .map_err(|e| format!("List stacks: {}", e))?;
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
            .current_dir(&tmp_dir)
            .output();
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
        self.update_backup_status("ok")
            .await
            .map_err(|e| format!("Status update: {}", e))?;
        tracing::info!("📦 Backup created: {}", output_path.display());
        self.append_logs(
            "system",
            &format!("📦 Backup created: {}\n", filename),
            "info",
        )
        .await
        .ok();
        Ok(output_path.to_string_lossy().to_string())
    }
}
