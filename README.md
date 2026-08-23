<picture>
  <source media="(prefers-color-scheme: dark)" srcset="public/icon.svg">
  <img alt="Dockpot" src="public/icon.svg" width="128" height="128" align="right">
</picture>

# 🐳 Dockpot

> Gestor de stacks Docker Compose con interfaz web, editor YAML, sincronización Git y control de versiones integrado.

Alternativa auto-hosteada a Dockge, escrita en **Rust + React**.

## ✨ Características

| | |
|---|---|
| 🖥️ **Gestión de stacks** | CRUD completo, start/stop/restart desde la web |
| ✏️ **Editor YAML** | Monaco Editor con syntax highlighting, validación en vivo |
| 📋 **Logs en vivo** | WebSocket con coloreado por severidad (ERROR/WARN) |
| 🔄 **Git sync** | Auto-commit, push/pull, diff visual tipo GitHub |
| 📦 **Plantillas** | 8 plantillas (nginx, postgres, redis, traefik, mariadb, mongodb, node-app, portainer) |
| 🔀 **Convertir docker run** | Pega un `docker run ...` y obtén compose.yaml |
| 🔐 **Auth OIDC** | Autenticación obligatoria via OpenID Connect |
| 🌙 **Modo oscuro** | Toggle con persistencia en localStorage |
| 📢 **Notificaciones** | Telegram, ntfy y Webhook |
| 💾 **Backup automático** | Programado con retención configurable |
| 📤 **Exportar** | Descarga stacks completos como zip |
| 📊 **Dashboard** | Resumen de stacks, Docker info, actividad reciente |

## 🚀 Inicio rápido

```bash
# Requisitos: Docker, Git

git clone https://github.com/atareao/dockpot
cd dockpot

# Configurar OIDC (obligatorio)
cp backend/.env.example .env
# Editar .env con tu proveedor OIDC (PocketID, Authelia, etc.)

# Construir y ejecutar
cd frontend && npm install && npm run build && cd ..
cd backend && cargo build --release
./target/release/dockpot
```

### Con Docker

```bash
docker run -d \
  --name dockpot \
  -p 3056:3056 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v dockpot-data:/app/data \
  -e OIDC_ISSUER_URL=https://auth.tudominio.com \
  -e OIDC_CLIENT_ID=dockpot \
  -e OIDC_CLIENT_SECRET=xxxx \
  -e OIDC_REDIRECT_URL=http://localhost:3056/auth/callback \
  ghcr.io/atareao/dockpot:latest
```

## 🛠️ Stack tecnológico

| Capa | Tecnología |
|------|------------|
| **Backend** | Rust + Axum + Tokio + SQLx (SQLite) |
| **Frontend** | React 19 + TypeScript + Ant Design 5 + Vite + Monaco Editor |
| **Docker** | Bollard (crate Rust para Docker Engine API) |
| **Git** | git2 (libgit2 nativo, sin dependencia de CLI) |
| **Auth** | OIDC con PKCE (PocketID-style) |

## 📋 API

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/health` | Health check (Docker + DB) |
| GET | `/api/stacks` | Listar stacks |
| POST | `/api/stacks` | Crear stack |
| GET/PUT/DELETE | `/api/stacks/{id}` | CRUD stack |
| POST | `/api/stacks/{id}/start` | Docker compose up |
| POST | `/api/stacks/{id}/stop` | Docker compose down |
| GET | `/api/stacks/{id}/compose` | Obtener compose.yaml |
| PUT | `/api/stacks/{id}/compose` | Actualizar compose.yaml |
| GET | `/api/stacks/{id}/logs/ws` | WebSocket logs en vivo |
| GET | `/api/stacks/{id}/sync` | Config Git sync |
| POST | `/api/templates/render` | Renderizar plantilla |
| POST | `/api/backup/run` | Ejecutar backup ahora |

## 📸 Capturas

![Dashboard](https://via.placeholder.com/800x450?text=Dockpot+Dashboard)
![Stack Detail](https://via.placeholder.com/800x450?text=Stack+Detail+with+Editor)

## 📄 Licencia

MIT &copy; 2026 [atareao](https://github.com/atareao)