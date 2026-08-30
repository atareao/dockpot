<picture>
  <source media="(prefers-color-scheme: dark)" srcset="public/icon.svg">
  <img alt="Dockpot" src="public/icon.svg" width="128" height="128" align="right">
</picture>

# 🐳 Dockpot

> Gestor de stacks Docker Compose con interfaz web, editor YAML, sincronización Git, +125 plantillas y descubrimiento automático de proyectos.

Alternativa auto-hosteada a Dockge, escrita en **Rust + React**.

## ✨ Características

| | |
|---|---|
| 📦 **Gestión de stacks** | CRUD completo, start/stop/restart con cards en grid responsive |
| 📄 **Compose + Env integrados** | Editor Monaco con validación YAML, ficheros .env en el mismo tab |
| 📋 **Logs en vivo** | SSE con resolución automática de nombres de contenedor |
| 🔄 **Git sync** | Auto-commit, push/pull, diff visual tipo GitHub |
| 🎨 **125 plantillas** | Biblioteca categorizada con búsqueda y filtros — nginx, postgres, ollama, nextcloud, jellyfin, gitea, vaultwarden, traefik, etc. |
| 🔍 **Discover** | Detecta proyectos compose externos y contenedores `docker run` |
| 📥 **Import & Capture** | Importa compose projects y captura contenedores standalone como stacks gestionados |
| 🔀 **Convertir docker run** | Pega un `docker run ...` y obtén compose.yaml |
| 🔐 **Auth OIDC** | Autenticación via OpenID Connect (opcional — modo dev sin auth) |
| 🌙 **Modo oscuro** | Toggle con persistencia en localStorage, tema sincronizado en toda la app |
| 📢 **Notificaciones** | Telegram, ntfy y Webhook |
| 💾 **Backup automático** | Programado con retención configurable |
| 📤 **Exportar** | Descarga stacks completos como zip |
| 📊 **Dashboard** | Tabs: My Stacks + Discover con cards de altura uniforme |
| 📱 **Mobile-first** | Diseño responsive, botones icon-only, dropdown para acciones secundarias |

## 🚀 Inicio rápido

### Con Podman (recomendado)

```bash
# Requisitos: podman, podman-compose, podman-docker

git clone https://github.com/atareao/dockpot
cd dockpot

# Sin OIDC → modo dev sin autenticación
podman compose up -d --build

# Abre http://localhost:3056
```

### Con Docker

```bash
docker compose up -d --build
```

### Con OIDC (producción)

Edita `compose.yml` o pasa variables de entorno:

```bash
OIDC_ISSUER_URL=https://auth.tudominio.com \
OIDC_CLIENT_ID=dockpot \
OIDC_CLIENT_SECRET=xxxx \
OIDC_REDIRECT_URL=http://localhost:3056/auth/callback \
podman compose up -d --build
```

### Desarrollo local (sin contenedor)

```bash
# Backend
cd backend && cargo run

# Frontend (hot reload, en otra terminal)
cd frontend && npm run dev
```

## 📸 Funcionalidades destacadas

### 📊 Dashboard con Tabs

```
┌──────────────────────────────────────────────────────┐
│ [🦆 Dockpot]  [📊 Dashboard]  [⚙️ Settings]  │ dev ☀️/🌙 🚪 │
├──────────────────────────────────────────────────────┤
│ 📦 My Stacks  │  🔍 Discover                         │
├──────────────────────────────────────────────────────┤
│ (stat cards: Total, Running, Stopped, Docker)        │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐              │
│ │ nginx    │ │ postgres │ │ ollama   │              │
│ │ running  │ │ stopped  │ │ running  │              │
│ │ ▶️ ⏹ 🔄 🗑 │ │ ▶️ ⏹ 🔄 🗑│ │ ▶️ ⏹ 🔄 🗑│              │
│ └──────────┘ └──────────┘ └──────────┘              │
├──────────────────────────────────────────────────────┤
│ Tab Discover:                                         │
│ 🔍 External Compose Projects  → [Import to Dockpot]  │
│ 📦 Standalone Containers       → [Capture as Stack]  │
└──────────────────────────────────────────────────────┘
```

### 📄 Vista de detalle

```
┌──────────────────────────────────────────────────────┐
│ ← nombre  [running]  ▶️ ⏹ 🔄 🚀 📥 ⋮               │
├──────── Info Card (siempre visible) ─────────────────┤
│ ID · Name · Status · Created · Updated               │
├──────── Tabs ────────────────────────────────────────┤
│ 📄 Compose  📋 Logs  📊 Stats  📢 Notifiers          │
│ ┌──────────────────────────────────────────────────┐ │
│ │ [Edit/Preview] • Validate • Save • Save & Deploy │ │
│ │ [Editor Monaco / YAML Preview]                   │ │
│ │ ──────────────────────────────────────────────── │ │
│ │ 🔤 Environment Files: [Add .env]                 │ │
│ │ .env  .env.production  .env.staging               │ │
│ └──────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────┘
```

### 🎯 Biblioteca de Plantillas (125 templates)

```
┌─ 📦 Template Library ─────────────────────────────────┐
│ 🔍 Search templates by name, description or category… │
│ ┌────────────────────────────────────────────────────┐│
│ │ 🤖 ai(4)  📈 analytics(2)  🗄️ database(9)  🛠️ dev(14) ││
│ │ 🎬 media(9)  ✅ productivity(15)  🔒 security(6)  … ││
│ └────────────────────────────────────────────────────┘│
│ ┌───────────┐  ┌── ⚙️ Configure | 👁️ Preview ───┐   │
│ │ ollama    │  │ Stack Name: [my-ollama________] │   │
│ │ whisper   │  │ Variables:                      │   │
│ │ tabby     │  │ OLLAMA_PORT: [11434           ] │   │
│ │ speakr    │  │ WEBUI_PORT: [8080             ] │   │
│ │ ...       │  │ GPU_ENABLED: [false           ] │   │
│ │           │  │ [🚀 Create Stack]               │   │
│ └───────────┘  └─────────────────────────────────┘   │
└──────────────────────────────────────────────────────┘
```

## 🗂️ Categorías de plantillas

| Categoría | Templates |
|---|---|
| 🧰 **tools** | 29 |
| ✅ **productivity** | 15 |
| 🛠️ **dev** | 14 |
| 📝 **content** | 11 |
| 🗄️ **database** | 9 |
| 🎬 **media** | 9 |
| 🔒 **security** | 6 |
| 📊 **monitoring** | 6 |
| ⚙️ **management** | 6 |
| 🤖 **ai** | 4 |
| 💬 **communication** | 4 |
| 🌐 **proxy** | 3 |
| ☁️ **cloud** | 2 |
| 📈 **analytics** | 2 |
| 💾 **storage** | 2 |
| 📱 **app** | 1 |
| 🔗 **middleware** | 1 |
| 🌍 **web** | 1 |

## 🛠️ Stack tecnológico

| Capa | Tecnología |
|------|------------|
| **Backend** | Rust + Axum 0.8 + Tokio + deadpool-sqlite + rusqlite |
| **Frontend** | React 19 + TypeScript + Ant Design 5 + Vite + Monaco Editor |
| **Docker** | Bollard (Rust Docker Engine API) |
| **Git** | git2 (libgit2 nativo, sin dependencia de CLI) |
| **Auth** | OIDC con PKCE (opcional) |
| **Templates** | YAML en disco, bind mount → editables sin recompilar |

## 📋 API

### Stacks

| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/stacks` | Listar stacks gestionados |
| `POST` | `/api/stacks` | Crear stack |
| `GET` | `/api/stacks/{id}` | Obtener stack |
| `PUT` | `/api/stacks/{id}` | Actualizar stack |
| `DELETE` | `/api/stacks/{id}` | Eliminar stack |
| `POST` | `/api/stacks/{id}/start` | `docker compose up -d` |
| `POST` | `/api/stacks/{id}/stop` | `docker compose down` |
| `POST` | `/api/stacks/{id}/restart` | Restart stack |
| `GET` | `/api/stacks/{id}/compose` | Obtener compose.yaml |
| `PUT` | `/api/stacks/{id}/compose` | Actualizar compose.yaml |
| `POST` | `/api/stacks/{id}/pull` | Pull images |

### Discover & Import

| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/stacks/discover` | Listar proyectos compose + contenedores externos |
| `POST` | `/api/stacks/import` | Importar proyecto compose externo |
| `POST` | `/api/stacks/create-from-container` | Capturar contenedor standalone como stack |

### Env Files

| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/stacks/{id}/env` | Listar env files |
| `PUT` | `/api/stacks/{id}/env` | Crear/actualizar env file |
| `DELETE` | `/api/stacks/{id}/env/{filename}` | Eliminar env file |

### Logs

| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/stacks/{id}/logs/ws` | SSE logs en vivo |
| `GET` | `/api/stacks/{id}/logs` | Historial de logs |

### Templates

| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/templates` | Listar todas las plantillas |
| `GET` | `/api/templates/{name}` | Obtener plantilla |
| `POST` | `/api/templates/render` | Renderizar plantilla con variables |

### Notificaciones

| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/notifiers` | Listar canales |
| `POST` | `/api/notifiers` | Crear canal |
| `PUT` | `/api/notifiers/{id}` | Actualizar canal |
| `DELETE` | `/api/notifiers/{id}` | Eliminar canal |
| `POST` | `/api/notifiers/{id}/test` | Probar canal |

### Otros

| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/health` | Health check |
| `GET` | `/api/docker/info` | Información de Docker |
| `GET` | `/api/stacks/{id}/stats` | Estadísticas del stack |
| `GET` | `/api/stacks/{id}/export` | Exportar stack como zip |
| `POST` | `/api/convert/docker-run` | Convertir `docker run` a compose |

## 🔧 Configuración

### Variables de entorno

| Variable | Default | Descripción |
|----------|---------|-------------|
| `HOST` | `0.0.0.0` | Dirección de bind |
| `PORT` | `3056` | Puerto HTTP |
| `DATA_DIR` | `./data` | Directorio de datos |
| `DATABASE_URL` | `./data/dockpot.db` | Ruta a SQLite |
| `STACKS_DIR` | `./data/stacks` | Directorio de stacks |
| `TEMPLATES_DIR` | `./templates` | Directorio de plantillas |
| `RUST_LOG` | `info` | Nivel de log |
| `LOG_FORMAT` | `pretty` | Formato de log |
| `OIDC_ISSUER_URL` | _(opcional)_ | URL del issuer OIDC (si no se setea → modo dev sin auth) |
| `OIDC_CLIENT_ID` | _(opcional)_ | Client ID OIDC |
| `OIDC_CLIENT_SECRET` | _(opcional)_ | Client secret OIDC |

### Modo dev (sin OIDC)

Si no se configuran las variables `OIDC_*`, Dockpot arranca en modo desarrollo sin autenticación:

```
⚠️  Modo dev sin OIDC — autenticación desactivada
🐳 Docker daemon reachable
🌐 Dockpot en http://[::]:3056
```

### Añadir plantillas personalizadas

Las plantillas se cargan desde `templates/*.yaml` en caliente. Solo necesitas:

1. Crear un fichero YAML con el formato adecuado
2. Refrescar la página en el navegador

Ejemplo:

```yaml
name: mi-servicio
description: "Descripción de mi servicio"
category: tools
variables:
  - name: PORT
    description: "Puerto HTTP"
    default: "8080"
    required: false
  - name: TAG
    description: "Versión de la imagen"
    default: "latest"
    required: false
compose: |
  services:
    app:
      image: myimage:${TAG:-latest}
      container_name: ${STACK_NAME}-app
      ports:
        - "${PORT:-8080}:8080"
      restart: unless-stopped
```

## 🏗️ Arquitectura del proyecto

```
/
├── backend/
│   └── src/
│       ├── main.rs           ← Bootstrap, router, Docker connect
│       ├── config.rs         ← Config desde env vars
│       ├── state.rs          ← AppState compartido
│       ├── auth.rs           ← OIDC middleware + dev mode bypass
│       ├── db.rs             ← SQLite operations
│       ├── containers.rs     ← Docker container listing
│       ├── templates.rs      ← Carga de templates YAML
│       ├── models.rs         ← Structs de datos
│       ├── routes/           ← Handlers por recurso
│       │   ├── stacks.rs     ← CRUD + discover + import + capture
│       │   ├── env.rs        ← Env files
│       │   ├── logs.rs       ← SSE log streaming
│       │   ├── templates.rs  ← Template endpoints
│       │   ├── sync.rs       ← Git sync
│       │   ├── backup.rs     ← Backups
│       │   └── notifiers.rs  ← Notificaciones
│       └── workers/
│           └── state.rs      ← Docker events + polling
├── frontend/
│   └── src/
│       ├── main.tsx          ← App root, ThemeContext, nav bar
│       ├── api/http.ts       ← API client unificado
│       ├── pages/
│       │   ├── Dashboard.tsx ← Tabs: My Stacks + Discover
│       │   ├── StackDetail.tsx ← Un Card con Tabs
│       │   └── Settings.tsx  ← Página de configuración
│       └── components/
│           ├── TemplateBrowser.tsx ← Modal con búsqueda y filtros
│           ├── Terminal.tsx   ← SSE log terminal
│           ├── YamlEditor.tsx ← Monaco editor
│           └── DiffViewer.tsx ← Git diff
├── templates/                ← 125 plantillas YAML
├── compose.yml               ← Podman/Docker Compose
├── Dockerfile                ← Multi-stage build
└── AGENTS.md                 ← Guía para asistentes IA
```

## 🧪 Tests

```bash
cd backend && cargo test
# 112+ tests — módulos: auth, config, containers, db, templates, state
```

## 🤝 Contribuir

1. Haz fork del repositorio
2. Crea una rama: `git checkout -b feature/mi-feature`
3. Haz cambios siguiendo el estilo del proyecto (consulta AGENTS.md)
4. Asegúrate de que `cargo test` y `cargo clippy` pasan
5. Crea un Pull Request

## 📄 Licencia

MIT &copy; 2026 [atareao](https://github.com/atareao)