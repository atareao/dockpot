CREATE TABLE IF NOT EXISTS backup_schedules (
    id TEXT PRIMARY KEY,
    enabled INTEGER NOT NULL DEFAULT 1,
    cron_expression TEXT NOT NULL DEFAULT '0 3 * * *',
    retention_days INTEGER NOT NULL DEFAULT 30,
    include_git INTEGER NOT NULL DEFAULT 1,
    include_env INTEGER NOT NULL DEFAULT 1,
    last_run_at TEXT,
    last_status TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);