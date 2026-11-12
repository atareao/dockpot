# Proposal: Fix Docker volume mount permission error

## Intent

Resolver el error `mkdir /app: permission denied` que ocurre cuando el daemon de Docker
(en el host) intenta crear directorios para bind mounts en rutas `/app/stacks/<uuid>/...`
que solo existen dentro del contenedor de Dockpot.

## Scope

- **Backend**: `config.rs`, `db.rs`, `routes/stacks.rs`
- **Infra**: `compose.yml`

## Impact

- Nuevo campo de configuración `STACKS_HOST_PATH` (default: `/var/lib/dockpot/stacks`)
- El volumen `dockpot-stacks` pasa de named volume a bind mount
- Las rutas en los compose files se reescriben automáticamente de `/app/stacks/` a la ruta host
- Los directorios necesarios se crean antes de ejecutar `docker compose up`

## Contratos

### Config (config.rs)

```rust
pub struct Config {
    // ... existing fields ...
    pub stacks_host_path: PathBuf,  // NEW: ruta host equivalente a STACKS_DIR
}
```

Nueva env var: `STACKS_HOST_PATH` (default: `/var/lib/dockpot/stacks`)

### Database (db.rs)

```rust
impl Database {
    /// Reescribe rutas /app/stacks/ a la ruta host en el contenido del compose file
    pub fn rewrite_stacks_paths(compose: &str, stacks_host_path: &str) -> String { ... }

    /// Crea los directorios necesarios para los bind mounts del compose file
    pub async fn ensure_volume_dirs(stack_path: &str, compose: &str) -> Result<()> { ... }
}
```

### Stacks routes (routes/stacks.rs)

- En `start()`: antes de `docker compose up`, llamar a `ensure_volume_dirs` y reescribir el compose file
- En `create_stack()` (db.rs): reescribir rutas al guardar el compose file

### compose.yml

```yaml
volumes:
  - /var/lib/dockpot/stacks:/app/stacks  # bind mount en lugar de named volume
```

## Escenarios BDD

### Escenario 1: Happy path — stack con volúmenes relativos
**Given** un stack creado desde template nginx con volúmenes `./nginx/conf.d` y `./nginx/html`
**When** se ejecuta `start_stack`
**Then** las rutas se reescriben a `/var/lib/dockpot/stacks/<uuid>/nginx/conf.d`
**And** los directorios se crean antes de `docker compose up`
**And** el comando `docker compose up -d` se ejecuta sin error

### Escenario 2: Stack sin volúmenes
**Given** un stack sin volúmenes bind mount
**When** se ejecuta `start_stack`
**Then** no se reescribe nada
**And** el compose file se mantiene intacto

### Escenario 3: Stack con rutas absolutas no relacionadas con stacks
**Given** un stack con volúmenes como `/data/external:/data`
**When** se ejecuta `start_stack`
**Then** las rutas no se modifican
**And** solo se reescriben las que contienen `/app/stacks/`

### Escenario 4: STACKS_HOST_PATH personalizado
**Given** `STACKS_HOST_PATH=/opt/dockpot/stacks`
**When** se crea un stack con volúmenes relativos
**Then** las rutas se reescriben a `/opt/dockpot/stacks/<uuid>/...`