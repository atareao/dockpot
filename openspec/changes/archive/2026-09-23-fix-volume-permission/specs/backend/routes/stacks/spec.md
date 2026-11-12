### Requirement: Pre-create directories before docker compose up

### ADDED

- `pub async fn prepare_compose_for_docker(compose: &str, stack_path: &str, stacks_host_path: &str) -> Result<String, String>` — prepara el compose para Docker reescribiendo rutas y creando directorios
- En `start()`: antes de `docker compose up -d`, se lee el compose de disco, se reescriben rutas, se crean directorios, y se escribe de vuelta

**Scenarios:**

1. **Start stack con volúmenes relativos**
   Given un stack con compose que tiene `./nginx/conf.d`
   When se llama a `start()`
   Then el compose file se reescribe con rutas host
   And los directorios se crean
   And `docker compose up -d` se ejecuta sin error

2. **Start stack sin volúmenes**
   Given un stack sin volúmenes bind mount
   When se llama a `start()`
   Then el compose file no se modifica
   And `docker compose up -d` se ejecuta sin error