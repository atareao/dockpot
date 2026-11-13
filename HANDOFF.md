# HANDOFF — Dockpot

## Estado actual

Todo en verde. PR #16 (fix-volume-permission) fusionado a `development`, hotfix #17 (pnpm-approve-builds) fusionado a `main` y sincronizado a `development`. Dockpot corriendo en local con Podman en `localhost:3056`.

## Rama activa

`development` — local y upstream sincronizados.

## Últimos cambios

- **fix-volume-permission** (`d273d3f`): nuevo config `STACKS_HOST_PATH`, reescritura de rutas `/app/stacks/` → host path, creación de directorios antes de `docker compose up`. 12 tests nuevos (135 total).
- **pnpm-approve-builds** (`2d2b5cd`): añadido `pnpm approve-builds esbuild` en Dockerfile para Alpine.
- **docs**: `docs/fix-volume-permission.md` + actualizada tabla env vars y common issues en `docs/project-reference.md`.

## Tarea pendiente

Ninguna. A la espera de nueva feature, bug o mejora.

## Para levantar en local

```bash
mkdir -p /data/dockpot-stacks
STACKS_HOST_PATH=/data/dockpot-stacks podman compose up -d --build
```

## Enlaces

- PR #16: https://github.com/atareao/dockpot/pull/16
- PR #17: https://github.com/atareao/dockpot/pull/17
