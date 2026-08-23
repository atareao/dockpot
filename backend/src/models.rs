use serde::{Deserialize, Serialize};

// ───── Stack ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stack {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub compose: String,
    pub status: String,
    pub path: String,
    pub created_at: String,
    pub updated_at: String,
}

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
    pub sync_type: String,
    pub remote_url: Option<String>,
    pub remote_branch: String,
    pub auth_token: Option<String>,
    pub last_commit: Option<String>,
    pub last_synced_at: Option<String>,
    pub status: String,
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

// ───── Dashboard / Docker Info ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStatus {
    pub total_stacks: u64,
    pub running_stacks: u64,
    pub stopped_stacks: u64,
    pub error_stacks: u64,
    pub docker_version: Option<String>,
    pub docker_containers: u64,
    pub docker_images: u64,
    pub recent_activity: Vec<RecentActivity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentActivity {
    pub stack_name: String,
    pub action: String,
    pub timestamp: String,
}

// ───── Env File ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvFile {
    pub id: String,
    pub stack_id: String,
    pub filename: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EnvFileRow {
    pub id: String,
    pub stack_id: String,
    pub filename: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<EnvFileRow> for EnvFile {
    fn from(row: EnvFileRow) -> Self {
        EnvFile {
            id: row.id,
            stack_id: row.stack_id,
            filename: row.filename,
            content: row.content,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

// ───── Notifier ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notifier {
    pub id: String,
    pub name: String,
    pub notifier_type: String,
    pub config_json: serde_json::Value,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NotifierRow {
    pub id: String,
    pub name: String,
    pub notifier_type: String,
    pub config_json: String,
    pub enabled: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<NotifierRow> for Notifier {
    fn from(row: NotifierRow) -> Self {
        Notifier {
            id: row.id,
            name: row.name,
            notifier_type: row.notifier_type,
            config_json: serde_json::from_str(&row.config_json).unwrap_or_default(),
            enabled: row.enabled != 0,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

// ───── Stack Stats ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackStats {
    pub stack_id: String,
    pub last_started_at: Option<String>,
    pub total_running_seconds: i64,
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

#[derive(Debug, Clone, Deserialize)]
pub struct CreateNotifierRequest {
    pub name: String,
    pub notifier_type: String,
    pub config_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DockerRunRequest {
    pub command: String,
    pub service_name: Option<String>,
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