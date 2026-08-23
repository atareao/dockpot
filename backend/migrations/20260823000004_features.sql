CREATE TABLE IF NOT EXISTS env_files (
    id TEXT PRIMARY KEY,
    stack_id TEXT NOT NULL REFERENCES stacks(id) ON DELETE CASCADE,
    filename TEXT NOT NULL DEFAULT '.env',
    content TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(stack_id, filename)
);

CREATE TABLE IF NOT EXISTS log_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stack_id TEXT NOT NULL REFERENCES stacks(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    level TEXT NOT NULL DEFAULT 'info',
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_logs_stack ON log_history(stack_id, created_at DESC);

CREATE TABLE IF NOT EXISTS notifiers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    notifier_type TEXT NOT NULL,
    config_json TEXT NOT NULL DEFAULT '{}',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS stack_notifiers (
    stack_id TEXT NOT NULL REFERENCES stacks(id) ON DELETE CASCADE,
    notifier_id TEXT NOT NULL REFERENCES notifiers(id) ON DELETE CASCADE,
    PRIMARY KEY (stack_id, notifier_id)
);

CREATE TABLE IF NOT EXISTS stack_stats (
    stack_id TEXT PRIMARY KEY REFERENCES stacks(id) ON DELETE CASCADE,
    last_started_at TEXT,
    total_running_seconds INTEGER NOT NULL DEFAULT 0
);