# AGENT DIRECTIVES: OPENSPEC (SDD) + TDD WORKFLOW

## ⚠️ REGLA DE ORO — LEER ANTES DE ACTUAR

**ANTES de escribir o editar CUALQUIER archivo de código fuente (Rust, TypeScript, JSX, CSS, etc.),
debes ejecutar `just check-spec` para confirmar que existe un change proposal aprobado.**

Si `just check-spec` falla:
1. DETENTE inmediatamente.
2. Informa al usuario que no hay un change proposal activo.
3. Pregunta si quiere crear uno con `openspec new change <feature>`.
4. NO escribas código hasta recibir aprobación explícita.

**SALTARSE ESTE PASO ES VIOLACIÓN DEL PROTOCOLO.**


---

## I. CORE PRINCIPLES & GOALS

- **Phase 0 — Legacy Support:** If modifying existing code without specs or tests, establish a baseline spec and characterization tests before introducing changes.
- **Phase 1 — SDD (OpenSpec):** No new code or tests may be written before a spec change proposal exists in `openspec/changes/<feature>/` and is approved by the user.
- **Phase 2 — TDD (Red-Green-Refactor):** Once the spec is approved, code MUST be developed strictly test-first using terminal commands.
- **Strict Verification:** Always run CLI test suites using terminal tools. Never assume code or tests pass/fail without CLI confirmation.

---

## IV. RESPONSE FORMAT & STATUS MESSAGES

Always prefix your progress updates with the current status tag:

```text
[LEGACY - INSPECT] Creating baseline spec & characterization tests.
[OPENSPEC - DRAFT] Generating change proposal in openspec/changes/...
[OPENSPEC - WAITING] Spec generated. Awaiting user review and approval.
[TDD - RED] Creating tests for scenario <Name> -> Running CLI tests.
[TDD - GREEN] Implementing minimal code -> Running CLI tests & type checks.
[TDD - REFACTOR] Refactoring code -> Running Clippy/ESLint & tests.
[OPENSPEC - ARCHIVE] Archiving change into openspec/specs/.
```

---

## Critical patterns

- **`!Send` lock scoping**: `deadpool-sqlite` returns a `SyncGuard` that is `!Send`. Any function that holds a DB lock across an `.await` MUST scope the lock with `{ let conn = obj.lock().unwrap(); /* sync work */ }` so it drops before the await. See `db.rs` `update_stack`, `upsert_env_file`, `delete_env_file`.
- **MethodRouter chaining**: Axum 0.8 sometimes can't infer types for `.put()` chained with `.get()` when handlers have 3+ extractors. Use `MethodRouter::new().get(fn).put(fn2)` instead of `routing::get(fn).put(fn2)`.
- **Docker compose discovery**: The `discover` endpoint runs `docker compose ls --format json` and `docker container ls --format ...`. Container names are resolved from compose project, not from stack names.
- **Logs resolution**: `logs_sse_handler` uses `docker compose ps --format '{{.Name}}'` to get actual container names instead of guessing from stack name.

---

### Dev Workflow

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

---

### Common Issues & Fixes

1. **"Handler not implemented"** in Axum routes → Use `MethodRouter::new()` with explicit type annotation: `let r: MethodRouter<AppState> = ...;`
2. **Docker events stream fails** with Podman → Worker has backoff (1s→5s) and falls back to polling after 5 failures
3. **Socket permissions** in container → Set `user: root:root` in compose.yml when using Podman rootless
4. **`!Send` error** with deadpool-sqlite → Scope the lock acquisition: `{ let conn = obj.lock().unwrap(); /* DB work */ }`
5. **Template YAML not loading** → Check `variables` is a list (`- name: X`) not a map (`X: value`)

---

## Dónde está el resto

| Tema | Fichero |
|---|---|
| Principios SDD/TDD y workflow por fases (secciones I–II) | `docs/sdd-tdd-workflow.md` |
| Lecciones OpenSpec (archive, headers, cascadas) | `docs/openspec-lessons.md` |
| Flujo git (gitflow, commits, releases) + convenciones del stack, arquitectura backend/frontend, UI, templates, env vars, issues comunes (secciones III–V) | `docs/project-reference.md` |
