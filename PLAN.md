# Dockpot — Plan de desarrollo

> Gestor de stacks Docker Compose con interfaz web, sincronización Git y control de versiones integrado.
>
> Alternativa auto-hosteada a Dockge (louislam), escrita en Rust + React.

## Stack tecnológico

| Capa | Tecnología |
|------|-----------|
| **Backend** | Rust + Axum + Tokio + SQLx (SQLite) |
| **Frontend** | React 19 + TypeScript + Ant Design 5 + Vite |
| **Docker** | `bollard` (crate Rust para Docker Engine API vía socket) |
| **Git** | `git2` (libgit2 nativo, sin dependencia de git CLI) |
| **YAML** | `serde_yaml` |
| **Auth** | OIDC (misma implementación que vigilatrs) |
| **SSE** | `tokio::sync::broadcast` + `axum::response::sse` |
| **Terminal** | WebSocket con `tokio-tungstenite` para logs interactivos |

## Estructura del proyecto

```
dockpot/
├── backend/
│   ├── migrations/
│   ├── src/
│   │   ├── main.rs
│   │   ├── config.rs
│   │   ├── auth.rs           # OIDC (copied from vigilatrs)
│   │   ├── db.rs
│   │   ├── models.rs
│   │   ├── routes/
│   │   │   ├── mod.rs
│   │   │   ├── auth_routes.rs
│   │   │   ├── stacks.rs    # CRUD + start/stop/restart
│   │   │   ├── sync.rs      # Git sync endpoints
│   │   │   ├── terminal.rs   # WebSocket logs
│   │   │   ├── settings.rs
│   │   │   └── status.rs
│   │   ├── docker/
│   │   │  ǵ── mod.rs          # Docker operations via ballard
│   │   │  ├── compose.rs      # Parse/enerate/validate mouth
│   │   │  └── logs.rs         # Stream logs
│   │   ├── git/
│   │   │  ├── mod.rs          # Git operations per stack
│   │   │  ├── commit.rs       # Auto-commit after changes
│   │   │  └── sync.rs         # Pull/push/diff/conflict detection
│   │   ── embed.rs            # include_dir! frontend dist
│   ├── Cargo.toml
├── frontend/
│   ├── src/
│   │   ├── main.tsx
│   │   ├── api/
│   │   │   └── http.ts
│   │   ├── pages/
│   │   │   ├── Dashboard.tsx
│   │   │   ├── Stacks.tsx      # Lista de stacks
│   │   │   ├── StackDetail.tsx # Editor compose.yaml + logs + sync
│   │   │   └── Settings.tsx
│   │   ├── components/
│   │   │   ├── YamlEditor.tsx  # Monaco Editor wrapper
│   │   │   └── Terminal.tsx    # WebSocket terminal
│   │   └── hooks/
│   │       ├── useAuth.ts
│   │       └── useTheme.ts
├── docker-compose.yml
├── Dockerfile
└── PLAN.md
```

## Fases

### Fase 1 — Esqueleto y stacks CRUD (1 sesión)

| Tarea | Archivos | Estimación |
|-------|----------|:----------:|
| 1.1 Scaffold proyecto + Cargo.toml | `Cargo.toml`, `main.rs`, `config.rs` | 20min |
| 1.2 Migración inicial SQLite + modelos | `migrations/`, `db.rs`, `mddels.rs` | 30min |
| 1.3 Auth OIDC (cpy from vigilatrs) | `auth.rs`, `routes/ath.rs` | 15min |
| 1.4 CRUD stacks + Docker operations (start/stop/restart) | `outes/stacks.rs`, `ocker/mod.rs` | 45min |
| 1.5 Serve embedded frontend | `embed.rs` | 5min |
| 1.6 Fronten: estructura + lista stacks + detalle con CRUD | `Stacks.tsx`, `StackDetil.ts`, `api/` | 40min |
| 1.7 Build test + git init + commi | — | 10min |
| **Total** | | **~2h 45min** |

### Fase 2 — Editor YAML + Monaco (1 sesión)

| Tarea | Archivos | Estimación |
|-------|----------|:----------:|
| 2.1 Endpoint GET/PUT /api/stacks/{id}/compose | `routes/stacks.rs` | 15min |
| 2.2 Validación compose con `erde_yaml` | `doker/compose.rs` | 10min |
|2.3 Monaco Editor wrapper en fronted | `YamlEditor.tsx` | 30min |
| 2.4 Check syntax con resaltado | Integrado en onaco | 5min|
| 2.5 Deploy button desde editor | `StackDetail.tsx` | 15min |
| **Total** | | **~1h 15min** |

### Fae 3 — Docker Compose operations (1 sesión)

| Tarea | Archivos | Estimación |
|-------|----------|:----------:|
| 3.1 Docker compose up/down/pull/logs via bollard | `docker/mod.rs`, `docker/compose.rs` | 40min |
| 3.2 Stream logs via WebSocket | `routes/terminal.rs`, `Terminal.tsx` | 30min |
| 3.3 Botón "Update images" | `routes/stacks.rs` | 10min |
| 3.4 UI: terminal en vivo en StackDetail | `Terminal.tsx`, `StackDetail.tsx` | 20min |
| **Total** | | **~1h 40min** |

### Fase 4 — Sincronización Git (1 sesión)

| Tarea | Archivos | Estimación |
|-------|----------|:----------:|
| 4.1 Tabla `stack_sync` en SQLite + modelo | `migrations/`, `models.rs`, `db.rs` | 20min |
| 4.2 Módulo git: open/clone/fetch/push/commit | `git/mod.rs`, `git/commit.rs`, `git/sync.rs` | 40min |
| 4.3 Auto-commit tras crear/editar/borrar stack | Hook en `routes/stacks.rs` | 15min |
| 4.4 Endpoint sync: pull/push/diff/confg | `outes/syn.rs` | 30min |
| 4.5 Scduler: git pull periódico con detección de cambios | `main.rs` scheduler loop | 15min |
| 4.6 Froten: modal configuración sync + badge estado | `tackDetail.tsx`, `tacks.tsx` | 30min |
| 4.7 Ver dif si conflicto | `StackDetail.tsx` | 15min |
| **Total** | | **~2h 45min** |

### Fase 5— Convertir docker run a compose yagentes (1 sesión)
|### Fase 5 — Convertir docker run a compose (1 sesión)
|\n|| Tarea | Archivos | Estimación |
||-------|---------|:----------:|
|| 5.1 Parse `docker run` args a compose.yaml | `convert.rs` | 30min |
|| 5.2 UI: input de `docker run ...` → vista previa → guardar | `Stacks.tsx` | 30min |
|| **Total** | | **~1h** |

## Tablas SQLite

### stacks
```sql
CREATE TABLE stacks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    compose TEXT NOT NULL DEFAULT 'version: "3"\nservices: {}',
    status TEXT NOT NULL DEFAULT 'stopped',  -- stopped | running | error
    path TEXT NOT NULL,                      -- ruta al directorio del stack
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### stack_sync
```sql
CREATE TABLE stack_sync (
    stack_id TEXT PRIMARY KEY REFERENCES stacks(id) ON DELETE CASCADE,
    sync_type TEXT NOT NULL DEFAULT 'none',  -- none | git_dir | git_remote
    remote_url TEXT,
    remote_branch TEXT NOT NULL DEFAULT 'main',
    auth_token TEXT,                          -- cifrado
    last_commit TEXT,
    last_synced_at TEXT,
    status TEXT NOT NULL DEFAULT 'idle'       -- idle | synced | pending | conflict
);
```

### settings
```sql
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

## Docker operations via bollard

| Operación | Bollard API |
|-----------|-------------|
| List images | `list_images()` |
| Pull image | `create_image()` con stream |
| List containers | `list_containers()` |
| Start compose | `docker compose up -d` vía `exec` o attach |
| Stop compose | `docker compose down` vía `exec` |
| Logs | `container_logs()` con stream → WebSocket |
| Inspect | `inspect_container()` |

## Endpoints API

### Públicos (sin auth)
| Método | Ruta | Función |
|--------|------|---------|
| GET | `/health` | Health check |
| GET | `/auth/login` | Redirect OIDC |
| GET | `/auth/callback` | OIDC callback |

### Protegidos (auth OIDC)
| Método | Ruta | Función |
|--------|------|---------|
| GET | `/api/me` | Info usuario |
| **Stacks** | | |
| GET | `/api/stacks` | List stacks |
| POST | `/api/stacks` | Create stack |
| GET | `/api/stacks/{id}` | Stack detail |
| PUT | `/api/stacks/{id}` | Update stack |
| DELETE | `/api/stacks/{id}` | Delete stack |
| POST | `/api/stacs/{id}/start` | Docker compose up -d |
| POST | `/api/stacs/{id}/stop` | Docker compse down |
| POST | `/api/stacks/{d}/restart` | Docker compose restart |
| POST | `/api/stacks/{id}/pull` | Docker compose pell (updateimages)|
| **Compose** | | |
| GET | `/api/stacks/{i}/compose` | Get compose.yaml content |
| PUT | `/api/stacks/{id}/compose` | Update compose.yam content |
| POST| `/api/stacks/{i}/validate` | Validate compose.yaml syntax |
| **Logs** | | |
| GET | `/api/stack/{id}/logs` | WebSocket upgrade → stream logs |
| **Git Sync** | | |
| GET | `/api/stacks/{id}/sync` | Get sync configuraton & status |
| PUT | `/api/stacs/{id}/sync` | Set sync configuron |
| POST | `/api/staks/{id}/sync/pll` | Git pull (fetch + mrge) |
| POST | `/api/staks/{id}/sync/push`| Git commit + push |
| GET | `/api/stacks/{d}/sync/diff` | Diff local vs remote (raw) |
| **nvert** | | |
| POST | `/api/convert/docker-un` | Parse `docer un` → compose.yaml |
| **Settngs** | | |
| GET | `/api/settings` | Get settings |
| POST | `/api/settings` | Save setting (key/val) |
| **Status** | | |
| GET | `/api/sttus` | Dashboard: stacks up/down, Docker info |

## Calidad

- `argo fmt --check && cargo clippy -- -D warnings && cargo test`
- `pnpm run build` (0 errores TS)
- Migraciones SQL con prefix `YYYYMMDDHHMMSS`
- Tests para cada endpoint nuevo
- Frontend build antes de cada commit de backend

## Post-MVP (futuro / descartado / pendiente)

### Descartado
- [-] Agentes remotos (gestión multi-host)
- [-] Status pages públicas

### Pendiente (implementado)

### Calidad de vida ✅
- ✅ Dashboard con cards de resumen (stacks up/down, última actividad, Docker info)
- ✅ Logs con coloreado por severidad (ERROR rojo, WARN amarillo)
- ✅ Logs históricos (persistencia en BD + endpoint GET)
- ✅ Exportar stack completo (zip con compose.yaml + .env + git)
- ✅ Modo oscuro (toggle sun/moon en UI, persistencia localStorage)

### Funcionalidad extra ✅
- ✅ Gestor de .env por stack (CRUD, persistencia en BD + disco)
- ✅ Docker info (versión engine, contenedores, imágenes, disco)
- ✅ Múltiples compose: soporte compose.yaml estándar
- ✅ Estadísticas de stack (último inicio, tiempo total running)

### Notificaciones ✅
- ✅ Tabla notifiers en SQLite
- ✅ Endpoints CRUD + test
- ✅ Telegram (vía API Bot)
- ✅ ntfy (vía HTTP con autenticación opcional)
- ✅ Asignación notifier → stack (N:M)
- ✅ Notificaciones automáticas al start/stop