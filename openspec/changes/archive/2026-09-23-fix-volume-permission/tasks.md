# Tasks: fix-volume-permission

## TDD Task Checklist

### Phase 1: Config

- [x] **RED**: Test `Config::load()` with STACKS_HOST_PATH unset → default `/var/lib/dockpot/stacks`
- [x] **RED**: Test `Config::load()` with STACKS_HOST_PATH=/opt/stacks → `/opt/stacks`
- [x] **GREEN**: Add `stacks_host_path` field to `Config` and `STACKS_HOST_PATH` env var
- [x] **REFACTOR**: Clean up, run `cargo clippy -- -D warnings`

### Phase 2: Path rewriting (db.rs)

- [x] **RED**: Test `rewrite_stacks_paths` with `/app/stacks/` paths → rewritten to host path
- [x] **RED**: Test `rewrite_stacks_paths` with non-`/app/stacks/` paths → unchanged
- [x] **RED**: Test `rewrite_stacks_paths` with empty string → empty
- [x] **GREEN**: Implement `rewrite_stacks_paths` in `Database`
- [x] **REFACTOR**: Clean up, run `cargo clippy -- -D warnings`

### Phase 3: Directory creation (db.rs)

- [x] **RED**: Test `ensure_volume_dirs` creates directories from compose volumes
- [x] **RED**: Test `ensure_volume_dirs` with no volumes → no directories created
- [x] **GREEN**: Implement `ensure_volume_dirs` in `Database`
- [x] **REFACTOR**: Clean up, run `cargo clippy -- -D warnings`

### Phase 4: Compose file rewriting in start()

- [x] **RED**: Test `start()` rewrites compose file before `docker compose up`
- [x] **GREEN**: Integrate `rewrite_stacks_paths` and `ensure_volume_dirs` in `start()`
- [x] **REFACTOR**: Clean up, run `cargo clippy -- -D warnings`

### Phase 5: compose.yml bind mount

- [x] Change `compose.yml` from named volume to bind mount
- [x] Add `STACKS_HOST_PATH` env var to compose.yml

### Phase 6: Integration

- [x] Run `cargo test` — all tests pass
- [x] Run `cargo clippy -- -D warnings` — zero warnings
- [x] Run `cargo fmt -- --check` — clean formatting