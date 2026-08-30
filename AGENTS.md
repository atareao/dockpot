# Dockpot — Project Guide for AI Agents

## Project Overview

Dockpot is a self-hosted Docker Compose stack manager with a web UI. Written in **Rust (Axum)** backend + **React (TypeScript, Ant Design 5)** frontend.

## Repository Structure

```
/ ─── frontend/     ← React 19 + Vite + TypeScript + Ant Design
      backend/      ← Rust + Axum + Tokio + SQLite (SQLx)
      templates/    ← 125 YAML template files for creating stacks
      compose.yml   ← Podman/Docker Compose for launching Dockpot itself
      Dockerfile    ← Multi-stage build (frontend → Rust → runtime)
```

## Git Flow

This project follows strict gitflow. See [GIT_FLOW.md](./GIT_FLOW.md) for:
- Branch structure (main, development, feature/*, hotfix/*)
- Conventional commits with gitmoji
- How to create features, hotfixes, and releases

### Branch naming rules
- Features: `feature/<kebab-case-name>`
- Hotfixes: `hotfix/<kebab-case-name>`

### Commit format

Use gitmoji + conventional commits:

| Tipo | Emoji | Bump |
|---|---|---|
| `feat` | ✨ | minor |
| `fix` | 🐛 | patch |
| `hotfix` | 🚑️ | patch |
| `docs` | 📝 | patch |
| `refactor` | ♻️ | patch |
| `style` | 💄 | patch |
| `chore` | 🔧 | patch |
| `ci` | 👷 | patch |

### Release process

1. PR `feature/*` → `development` (review)
2. PR `development` → `main` (release)
3. After merge to main: tag with `vX.Y.Z` and sync development

## Backend Architecture

| Layer | Details |
|---|---|
| **Web** | Axum 0.8 (Router, extractors, middleware) |
| **DB** | SQLite via `deadpool-sqlite` + `rusqlite` |
| **Docker** | Bollard (Rust Docker Engine API) |
| **Git** | git2 (libgit2 native bindings) |
| **Auth** | OIDC with PKCE (optional — dev mode without OIDC) |
| **Templates** | YAML files loaded from `templates/` directory at runtime |

### Key modules (`backend/src/`)

| File | Purpose |
|---|---|
| `main.rs` | App bootstrap, Docker connect, OIDC setup, router wiring |
| `config.rs` | Env-based config with `env_required` / `env_optional` |
| `state.rs` | `AppState` — shared state (Docker, DB, channels, OIDC) |
| `auth.rs` | OIDC middleware, JWT validation, dev mode bypass |
| `db.rs` | SQLite operations (stacks, env_files, notifiers, backups, stats) |
| `models.rs` | Serde structs for API request/response |
| `containers.rs` | Docker container listing, state detection |
| `templates.rs` | Template loading from YAML, variable substitution |
| `routes/stacks.rs` | Stack CRUD, discover, import, create-from-container |
| `routes/env.rs` | Env file CRUD |
| `routes/logs.rs` | SSE log streaming (resolves container names from compose) |
| `routes/templates.rs` | Template API endpoints |
| `routes/sync.rs` | Git sync config, pull, push, diff |
| `routes/backup.rs` | Backup scheduling and execution |
| `routes/notifiers.rs` | Notification channel CRUD |
| `workers/state.rs` | Docker events stream → polling fallback with backoff |

### Critical patterns

- **`!Send` lock scoping**: `deadpool-sqlite` returns a `SyncGuard` that is `!Send`. Any function that holds a DB lock across an `.await` MUST scope the lock with `{ let conn = obj.lock().unwrap(); /* sync work */ }` so it drops before the await. See `db.rs` `update_stack`, `upsert_env_file`, `delete_env_file`.
- **MethodRouter chaining**: Axum 0.8 sometimes can't infer types for `.put()` chained with `.get()` when handlers have 3+ extractors. Use `MethodRouter::new().get(fn).put(fn2)` instead of `routing::get(fn).put(fn2)`.
- **Docker compose discovery**: The `discover` endpoint runs `docker compose ls --format json` and `docker container ls --format ...`. Container names are resolved from compose project, not from stack names.
- **Logs resolution**: `logs_sse_handler` uses `docker compose ps --format '{{.Name}}'` to get actual container names instead of guessing from stack name.

## Frontend Architecture

| Technology | Version |
|---|---|
| React | 19 |
| TypeScript | 5.x |
| Ant Design | 5.x |
| Vite | Latest |
| Monaco Editor | YAML editor |
| React Router | v6 |

### Key files (`frontend/src/`)

| File | Purpose |
|---|---|
| `main.tsx` | App root, ThemeContext, top navigation bar (no sidebar), routes |
| `api/http.ts` | API client — all `api.*` method calls |
| `pages/Dashboard.tsx` | Tabs: 📦 My Stacks + 🔍 Discover (external projects & containers) |
| `pages/StackDetail.tsx` | Single Card with Tabs: 📄 Compose(+Env), 📋 Logs, 📊 Stats, 📢 Notifiers |
| `pages/Settings.tsx` | Settings page with Tabs (🔔 Notifiers) |
| `components/TemplateBrowser.tsx` | Modal with search, category pills, config/preview tabs, scrollbar styling |
| `components/Terminal.tsx` | SSE log terminal with color-coded output |
| `components/YamlEditor.tsx` | Monaco-based YAML editor with validation |

### UI patterns

- **Mobile-first**: padding 12px, icon-only buttons on small screens, dropdown "⋮" for less-used actions
- **Dark mode**: `ThemeContext` exported from `main.tsx`, body background sync'd, no nested `minHeight: 100vh` Layouts
- **Tabs everywhere**: Dashboard (Stacks/Discover), StackDetail (Compose/Logs/Stats/Notifiers), TemplateBrowser (Configure/Preview)
- **Scrollbar styling**: Class `dp-scroll` with thin (5px), rounded, semi-transparent scrollbars

## Templates System

125 templates stored as YAML in `templates/`. Each file:

```yaml
name: template-name
description: "Description"
category: "category-name"
variables:
  - name: VAR_NAME
    description: "What it does"
    default: "default-value"
    required: false
compose: |
  services:
    app:
      image: image:${VAR_TAG:-latest}
      container_name: ${STACK_NAME}-app
```

- Mounted as bind mount in compose.yml: `./templates:/app/templates:ro`
- **No recompilation needed** — edit YAML files and refresh the browser
- Variables MUST be a YAML list, not a map

## Dev Workflow

```bash
# Build and run with Podman
podman compose up -d --build

# Or run backend directly for faster iteration
cd backend && cargo run

# Frontend dev (hot reload)
cd frontend && npm run dev

# TypeScript check
cd frontend && npx tsc --noEmit

# Rust tests
cd backend && cargo test
```

## Environment Variables

| Variable | Default | Purpose |
|---|---|---|
| `HOST` | `0.0.0.0` | HTTP bind address |
| `PORT` | `3056` | HTTP port |
| `DATA_DIR` | `./data` | Data directory |
| `DATABASE_URL` | `./data/dockpot.db` | SQLite file path |
| `STACKS_DIR` | `./data/stacks` | Stack compose file storage |
| `TEMPLATES_DIR` | `./templates` | Template YAML directory |
| `RUST_LOG` | `info` | Log level |
| `LOG_FORMAT` | `pretty` | Log format |
| `OIDC_ISSUER_URL` | _(optional)_ | OIDC issuer — leave unset for dev mode (no auth) |
| `OIDC_CLIENT_ID` | _(optional)_ | OIDC client ID |
| `OIDC_CLIENT_SECRET` | _(optional)_ | OIDC client secret |

## Common Issues & Fixes

1. **"Handler not implemented"** in Axum routes → Use `MethodRouter::new()` with explicit type annotation: `let r: MethodRouter<AppState> = ...;`
2. **Docker events stream fails** with Podman → Worker has backoff (1s→5s) and falls back to polling after 5 failures
3. **Socket permissions** in container → Set `user: root:root` in compose.yml when using Podman rootless
4. **`!Send` error** with deadpool-sqlite → Scope the lock acquisition: `{ let conn = obj.lock().unwrap(); /* DB work */ }`
5. **Template YAML not loading** → Check `variables` is a list (`- name: X`) not a map (`X: value`)