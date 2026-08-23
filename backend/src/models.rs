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

// ───── Backup Schedule ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSchedule {
    pub id: String,
    pub enabled: bool,
    pub cron_expression: String,
    pub retention_days: i64,
    pub include_git: bool,
    pub include_env: bool,
    pub last_run_at: Option<String>,
    pub last_status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BackupScheduleRow {
    pub id: String,
    pub enabled: i32,
    pub cron_expression: String,
    pub retention_days: i64,
    pub include_git: i32,
    pub include_env: i32,
    pub last_run_at: Option<String>,
    pub last_status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<BackupScheduleRow> for BackupSchedule {
    fn from(row: BackupScheduleRow) -> Self {
        BackupSchedule {
            id: row.id,
            enabled: row.enabled != 0,
            cron_expression: row.cron_expression,
            retention_days: row.retention_days,
            include_git: row.include_git != 0,
            include_env: row.include_env != 0,
            last_run_at: row.last_run_at,
            last_status: row.last_status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

// ───── Tests ─────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Stack ──

    #[test]
    fn test_stack_from_row() {
        let row = StackRow {
            id: "abc-123".into(),
            name: "test-stack".into(),
            description: Some("A test".into()),
            compose: "services:\n  app:\n    image: nginx".into(),
            status: "running".into(),
            path: "/data/stacks/abc-123".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let stack: Stack = row.into();
        assert_eq!(stack.name, "test-stack");
        assert_eq!(stack.description, Some("A test".into()));
        assert_eq!(stack.status, "running");
    }

    #[test]
    fn test_stack_default_status() {
        let row = StackRow {
            id: "x".into(), name: "x".into(), description: None,
            compose: String::new(), status: "stopped".into(),
            path: "/tmp".into(), created_at: String::new(), updated_at: String::new(),
        };
        let stack: Stack = row.into();
        assert_eq!(stack.status, "stopped");
    }

    // ── StackSync ──

    #[test]
    fn test_stack_sync_from_row() {
        let row = StackSyncRow {
            stack_id: "s1".into(),
            sync_type: "git_remote".into(),
            remote_url: Some("https://github.com/user/repo.git".into()),
            remote_branch: "main".into(),
            auth_token: None,
            last_commit: Some("abc123".into()),
            last_synced_at: Some("2026-01-01T00:00:00Z".into()),
            status: "synced".into(),
        };
        let sync: StackSync = row.into();
        assert_eq!(sync.sync_type, "git_remote");
        assert_eq!(sync.remote_url.unwrap(), "https://github.com/user/repo.git");
        assert_eq!(sync.status, "synced");
    }

    #[test]
    fn test_sync_default_branch() {
        let row = StackSyncRow {
            stack_id: "s2".into(), sync_type: "git_dir".into(),
            remote_url: None, remote_branch: "main".into(),
            auth_token: None, last_commit: None, last_synced_at: None,
            status: "idle".into(),
        };
        let sync: StackSync = row.into();
        assert_eq!(sync.remote_branch, "main");
        assert!(sync.last_commit.is_none());
    }

    // ── Notifier ──

    #[test]
    fn test_notifier_from_row() {
        let row = NotifierRow {
            id: "n1".into(), name: "Telegram Alerts".into(),
            notifier_type: "telegram".into(),
            config_json: r#"{"bot_token":"xxx"}"#.into(),
            enabled: 1, created_at: String::new(), updated_at: String::new(),
        };
        let n: Notifier = row.into();
        assert_eq!(n.name, "Telegram Alerts");
        assert_eq!(n.notifier_type, "telegram");
        assert!(n.enabled);
    }

    #[test]
    fn test_notifier_disabled() {
        let row = NotifierRow {
            id: "n2".into(), name: "Disabled".into(),
            notifier_type: "ntfy".into(), config_json: "{}".into(),
            enabled: 0, created_at: String::new(), updated_at: String::new(),
        };
        let n: Notifier = row.into();
        assert!(!n.enabled);
    }

    #[test]
    fn test_notifier_default_config() {
        let row = NotifierRow {
            id: "n3".into(), name: "Empty".into(),
            notifier_type: "webhook".into(), config_json: "not-json".into(),
            enabled: 1, created_at: String::new(), updated_at: String::new(),
        };
        let n: Notifier = row.into();
        // Falls back to Null on invalid JSON
        assert_eq!(n.config_json, serde_json::Value::Null);
    }

    // ── Agent ──

    #[test]
    fn test_agent_from_row() {
        let row = AgentRow {
            id: "a1".into(), name: "docker-01".into(),
            agent_type: "docker".into(), host: "192.168.1.100".into(),
            port: 2376, tls_enabled: 1,
            ca_cert: Some("cert".into()), client_cert: None, client_key: None,
            description: Some("Main host".into()), enabled: 1,
            created_at: String::new(), updated_at: String::new(),
        };
        let a: Agent = row.into();
        assert_eq!(a.name, "docker-01");
        assert_eq!(a.host, "192.168.1.100");
        assert_eq!(a.port, 2376);
        assert!(a.tls_enabled);
        assert!(a.enabled);
    }

    #[test]
    fn test_agent_disabled() {
        let row = AgentRow {
            id: "a2".into(), name: "offline".into(),
            agent_type: "docker".into(), host: "10.0.0.1".into(),
            port: 2375, tls_enabled: 0,
            ca_cert: None, client_cert: None, client_key: None,
            description: None, enabled: 0,
            created_at: String::new(), updated_at: String::new(),
        };
        let a: Agent = row.into();
        assert!(!a.enabled);
        assert!(!a.tls_enabled);
    }

    // ── DashboardStatus ──

    #[test]
    fn test_dashboard_status_serialize() {
        let ds = DashboardStatus {
            total_stacks: 5, running_stacks: 3, stopped_stacks: 2,
            error_stacks: 0, docker_version: Some("24.0.7".into()),
            docker_containers: 10, docker_images: 25,
            recent_activity: vec![],
        };
        let json = serde_json::to_string(&ds).unwrap();
        assert!(json.contains("\"total_stacks\":5"));
        assert!(json.contains("\"docker_version\":\"24.0.7\""));
    }

    // ── EnvFile ──

    #[test]
    fn test_env_file_from_row() {
        let row = EnvFileRow {
            id: "e1".into(), stack_id: "s1".into(),
            filename: ".env".into(), content: "DB_HOST=localhost".into(),
            created_at: String::new(), updated_at: String::new(),
        };
        let env: EnvFile = row.into();
        assert_eq!(env.filename, ".env");
        assert_eq!(env.content, "DB_HOST=localhost");
    }

    // ── Requests ──

    #[test]
    fn test_create_stack_request_deserialize() {
        let json = r#"{"name": "myapp", "description": "My app", "compose": "services:\n  app:\n    image: nginx"}"#;
        let req: CreateStackRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "myapp");
        assert_eq!(req.description.unwrap(), "My app");
    }

    #[test]
    fn test_create_stack_request_minimal() {
        let json = r#"{"name": "myapp"}"#;
        let req: CreateStackRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "myapp");
        assert!(req.compose.is_none());
    }

    #[test]
    fn test_sync_config_request_defaults() {
        let req = SyncConfigRequest {
            sync_type: Some("git_remote".into()),
            remote_url: Some("https://github.com/u/r.git".into()),
            remote_branch: None,
            auth_token: None,
        };
        assert_eq!(req.sync_type.unwrap(), "git_remote");
    }

    #[test]
    fn test_git_diff_serialize() {
        let diff = GitDiff {
            files_changed: vec!["compose.yaml".into()],
            additions: 5,
            deletions: 2,
            diff_text: "+hello\n-world\n".into(),
        };
        let json = serde_json::to_string(&diff).unwrap();
        assert!(json.contains("\"additions\":5"));
    }

    // ── StackStats ──

    #[test]
    fn test_stack_stats_serialize() {
        let stats = StackStats {
            stack_id: "s1".into(),
            last_started_at: Some("2026-01-01T00:00:00Z".into()),
            total_running_seconds: 3600,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_running_seconds\":3600"));
    }
}