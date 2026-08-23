CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    agent_type TEXT NOT NULL DEFAULT 'docker',
    host TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 2376,
    tls_enabled INTEGER NOT NULL DEFAULT 1,
    ca_cert TEXT,
    client_cert TEXT,
    client_key TEXT,
    description TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);