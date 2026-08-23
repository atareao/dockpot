-- Create initial tables for Dockpot stack manager
CREATE TABLE IF NOT EXISTS stacks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    compose TEXT NOT NULL DEFAULT 'version: "3"\nservices: {}',
    status TEXT NOT NULL DEFAULT 'stopped',
    path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);