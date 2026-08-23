use serde::{Deserialize, Serialize};

// ───── Stack ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stack {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub compose: String,
    pub status: String, // stopped | running | error
    pub path: String,
    pub created_at: String,
    pub updated_at: String,
}

// ───── DB row type ─────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StackRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub compose: String,
    pub status: String,
    pub path: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<StackRow> for Stack {
    fn from(row: StackRow) -> Self {
        Stack {
            id: row.id,
            name: row.name,
            description: row.description,
            compose: row.compose,
            status: row.status,
            path: row.path,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

// ───── Stack Sync ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackSync {
    pub stack_id: String,
    pub sync_type: String,    // none | git_dir | git_remote
    pub remote_url: Option<String>,
    pub remote_branch: String,
    pub auth_token: Option<String>, // stored encrypted (TODO)
    pub last_commit: Option<String>,
    pub last_synced_at: Option<String>,
    pub status: String,       // idle | synced | pending | conflict
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StackSyncRow {
    pub stack_id: String,
    pub sync_type: String,
    pub remote_url: Option<String>,
    pub remote_branch: String,
    pub auth_token: Option<String>,
    pub last_commit: Option<String>,
    pub last_synced_at: Option<String>,
    pub status: String,
}

impl From<StackSyncRow> for StackSync {
    fn from(row: StackSyncRow) -> Self {
        StackSync {
            stack_id: row.stack_id,
            sync_type: row.sync_type,
            remote_url: row.remote_url,
            remote_branch: row.remote_branch,
            auth_token: row.auth_token,
            last_commit: row.last_commit,
            last_synced_at: row.last_synced_at,
            status: row.status,
        }
    }
}

// ───── Dashboard status ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStatus {
    pub total_stacks: u64,
    pub running_stacks: u64,
    pub stopped_stacks: u64,
}

// ───── Create / Update requests ─────

#[derive(Debug, Clone, Deserialize)]
pub struct CreateStackRequest {
    pub name: String,
    pub description: Option<String>,
    pub compose: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateStackRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub compose: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyncConfigRequest {
    pub sync_type: Option<String>,
    pub remote_url: Option<String>,
    pub remote_branch: Option<String>,
    pub auth_token: Option<String>,
}

// ───── Git diff output ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiff {
    pub files_changed: Vec<String>,
    pub additions: u64,
    pub deletions: u64,
    pub diff_text: String,
}

// ───── Agent ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub host: String,
    pub port: u16,
    pub tls_enabled: bool,
    pub ca_cert: Option<String>,
    pub client_cert: Option<String>,
    pub client_key: Option<String>,
    pub description: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentRow {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub host: String,
    pub port: i64,
    pub tls_enabled: i32,
    pub ca_cert: Option<String>,
    pub client_cert: Option<String>,
    pub client_key: Option<String>,
    pub description: Option<String>,
    pub enabled: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<AgentRow> for Agent {
    fn from(row: AgentRow) -> Self {
        Agent {
            id: row.id,
            name: row.name,
            agent_type: row.agent_type,
            host: row.host,
            port: row.port as u16,
            tls_enabled: row.tls_enabled != 0,
            ca_cert: row.ca_cert,
            client_cert: row.client_cert,
            client_key: row.client_key,
            description: row.description,
            enabled: row.enabled != 0,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub host: String,
    pub port: Option<u16>,
    pub tls_enabled: Option<bool>,
    pub ca_cert: Option<String>,
    pub client_cert: Option<String>,
    pub client_key: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DockerRunRequest {
    pub command: String,
    pub service_name: Option<String>,
}