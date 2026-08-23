use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{DashboardStatus, Stack, StackRow, StackSync, StackSyncRow};

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
        sqlx::raw_sql(tables)
            .execute(&pool)
            .await
            .context("Failed to run initial migration")?;
        let sync_table = include_str!("../migrations/20260823000002_sync.sql");
        sqlx::raw_sql(sync_table)
            .execute(&pool)
            .await
            .context("Failed to run sync migration")?;

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
        .await
        .context("Failed to list stacks")?;
        Ok(rows.into_iter().map(Stack::from).collect())
    }

    pub async fn get_stack(&self, id: &str) -> Result<Option<Stack>> {
        let row = sqlx::query_as::<_, StackRow>(
            "SELECT id, name, description, compose, status, path, created_at, updated_at \
             FROM stacks WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get stack")?;
        Ok(row.map(Stack::from))
    }

    pub async fn get_stack_by_name(&self, name: &str) -> Result<Option<Stack>> {
        let row = sqlx::query_as::<_, StackRow>(
            "SELECT id, name, description, compose, status, path, created_at, updated_at \
             FROM stacks WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get stack by name")?;
        Ok(row.map(Stack::from))
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

        tokio::fs::create_dir_all(&stacks_path)
            .await
            .context("Failed to create stack directory")?;

        let compose_path = stacks_path.join("compose.yaml");
        tokio::fs::write(&compose_path, compose)
            .await
            .context("Failed to write compose file")?;

        let path_str = stacks_path.to_string_lossy().to_string();

        sqlx::query(
            "INSERT INTO stacks (id, name, description, compose, status, path, created_at, updated_at) \
             VALUES (?, ?, ?, ?, 'stopped', ?, ?, ?)",
        )
        .bind(&id)
        .bind(name)
        .bind(description)
        .bind(compose)
        .bind(&path_str)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .context("Failed to insert stack")?;

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

    pub async fn update_stack(&self, id: &str, name: &str, description: Option<&str>, compose: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let rows = sqlx::query(
            "UPDATE stacks SET name=?, description=?, compose=?, updated_at=? WHERE id=?",
        )
        .bind(name)
        .bind(description)
        .bind(compose)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update stack")?;

        if let Ok(Some(stack)) = self.get_stack(id).await {
            let compose_path = Path::new(&stack.path).join("compose.yaml");
            let _ = tokio::fs::write(&compose_path, compose).await;
        }

        Ok(rows.rows_affected() > 0)
    }

    pub async fn update_status(&self, id: &str, status: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let rows = sqlx::query("UPDATE stacks SET status=?, updated_at=? WHERE id=?")
            .bind(status)
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to update stack status")?;
        Ok(rows.rows_affected() > 0)
    }

    pub async fn delete_stack(&self, id: &str) -> Result<bool> {
        if let Ok(Some(stack)) = self.get_stack(id).await {
            let _ = tokio::fs::remove_dir_all(&stack.path).await;
        }

        let rows = sqlx::query("DELETE FROM stacks WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete stack")?;
        Ok(rows.rows_affected() > 0)
    }

    // ───── Stack Sync ─────

    pub async fn get_sync_config(&self, stack_id: &str) -> Result<Option<StackSync>> {
        let row = sqlx::query_as::<_, StackSyncRow>(
            "SELECT stack_id, sync_type, remote_url, remote_branch, auth_token, last_commit, last_synced_at, status \
             FROM stack_sync WHERE stack_id = ?",
        )
        .bind(stack_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get sync config")?;
        Ok(row.map(StackSync::from))
    }

    pub async fn upsert_sync_config(&self, sync: &StackSync) -> Result<()> {
        sqlx::query(
            "INSERT INTO stack_sync (stack_id, sync_type, remote_url, remote_branch, auth_token, last_commit, last_synced_at, status) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(stack_id) DO UPDATE SET \
             sync_type=excluded.sync_type, remote_url=excluded.remote_url, \
             remote_branch=excluded.remote_branch, auth_token=excluded.auth_token, \
             last_commit=excluded.last_commit, last_synced_at=excluded.last_synced_at, \
             status=excluded.status",
        )
        .bind(&sync.stack_id)
        .bind(&sync.sync_type)
        .bind(&sync.remote_url)
        .bind(&sync.remote_branch)
        .bind(&sync.auth_token)
        .bind(&sync.last_commit)
        .bind(&sync.last_synced_at)
        .bind(&sync.status)
        .execute(&self.pool)
        .await
        .context("Failed to upsert sync config")?;
        Ok(())
    }

    pub async fn update_sync_status(&self, stack_id: &str, status: &str, last_commit: Option<&str>) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE stack_sync SET status=?, last_commit=COALESCE(?, last_commit), last_synced_at=? WHERE stack_id=?",
        )
        .bind(status)
        .bind(last_commit)
        .bind(&now)
        .bind(stack_id)
        .execute(&self.pool)
        .await
        .context("Failed to update sync status")?;
        Ok(())
    }

    pub async fn list_sync_configs(&self) -> Result<Vec<StackSync>> {
        let rows = sqlx::query_as::<_, StackSyncRow>(
            "SELECT stack_id, sync_type, remote_url, remote_branch, auth_token, last_commit, last_synced_at, status \
             FROM stack_sync ORDER BY stack_id",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list sync configs")?;
        Ok(rows.into_iter().map(StackSync::from).collect())
    }

    // ───── Dashboard ─────

    pub async fn get_dashboard_status(&self) -> Result<DashboardStatus> {
        let stacks = self.list_stacks().await?;
        let total = stacks.len() as u64;
        let running = stacks.iter().filter(|s| s.status == "running").count() as u64;
        let stopped = stacks.iter().filter(|s| s.status == "stopped").count() as u64;

        Ok(DashboardStatus {
            total_stacks: total,
            running_stacks: running,
            stopped_stacks: stopped,
        })
    }

    // ───── Settings ─────

    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        sqlx::query_scalar("SELECT value FROM settings WHERE key=?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to get setting")
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await
            .context("Failed to set setting")?;
        Ok(())
    }
}