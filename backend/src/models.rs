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