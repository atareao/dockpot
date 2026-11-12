### Requirement: Path rewriting for Docker volume mounts

### ADDED

- `pub fn rewrite_stacks_paths(compose: &str, stacks_host_path: &str) -> String` — reescribe rutas `/app/stacks/` a la ruta host
- `pub async fn ensure_volume_dirs(stack_path: &str, compose: &str) -> std::io::Result<()>` — crea directorios necesarios para bind mounts

**Scenarios:**

1. **Compose con rutas relativas resueltas a /app/stacks/**
   Given compose content con `- ./nginx/conf.d:/etc/nginx/conf.d:ro`
   And `stacks_host_path = "/var/lib/dockpot/stacks"`
   And `stack_path = "/app/stacks/uuid-123"`
   When se llama a `rewrite_stacks_paths`
   Then devuelve `- /var/lib/dockpot/stacks/uuid-123/nginx/conf.d:/etc/nginx/conf.d:ro`

2. **Compose sin rutas /app/stacks/**
   Given compose content con `- /data/external:/data:rw`
   When se llama a `rewrite_stacks_paths`
   Then el contenido no se modifica

3. **Compose vacío**
   Given compose content vacío
   When se llama a `rewrite_stacks_paths`
   Then devuelve string vacío

4. **ensure_volume_dirs crea directorios**
   Given compose con `- /app/stacks/uuid-123/nginx/conf.d:/etc/nginx/conf.d:ro`
   When se llama a `ensure_volume_dirs`
   Then se crea el directorio `/app/stacks/uuid-123/nginx/conf.d`