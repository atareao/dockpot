CREATE TABLE IF NOT EXISTS stack_sync (
    stack_id TEXT PRIMARY KEY REFERENCES stacks(id) ON DELETE CASCADE,
    sync_type TEXT NOT NULL DEFAULT 'none',
    remote_url TEXT,
    remote_branch TEXT NOT NULL DEFAULT 'main',
    auth_token TEXT,
    last_commit TEXT,
    last_synced_at TEXT,
    status TEXT NOT NULL DEFAULT 'idle'
);