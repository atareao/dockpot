### Requirement: STACKS_HOST_PATH configuration

### ADDED

- `pub stacks_host_path: PathBuf` field added to `Config` struct
- `STACKS_HOST_PATH` env var (default: `/var/lib/dockpot/stacks`)

**Scenarios:**

1. **STACKS_HOST_PATH no definido**
   Given `STACKS_HOST_PATH` no está en el entorno
   When se carga `Config::load()`
   Then `stacks_host_path` es `/var/lib/dockpot/stacks`

2. **STACKS_HOST_PATH personalizado**
   Given `STACKS_HOST_PATH=/opt/mis/stacks`
   When se carga `Config::load()`
   Then `stacks_host_path` es `/opt/mis/stacks`

3. **STACKS_HOST_PATH vacío**
   Given `STACKS_HOST_PATH=""`
   When se carga `Config::load()`
   Then `stacks_host_path` es el default `/var/lib/dockpot/stacks`