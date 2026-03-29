---
description: Single-contract scope for shipping a baked backend.toml default in the backend image and overriding config through docker-compose environment variables.
---

# backend-env-config-2026-03-29 Contract

## Context
- The current Docker deployment shape requires bind-mounting `deploy/backend.toml` into `/etc/chronicle-engine-backend/backend.toml`.
- The desired deploy shape is simpler: the image should already contain a default config file, and operators should override keys through `docker-compose.yml` `environment:` entries instead of maintaining a host-mounted TOML file by default.

## Findings
- `backend/src/main.rs` loads config from `/etc/chronicle-engine-backend/backend.toml`.
- `deploy/Dockerfile` does not currently copy a default `backend.toml` into the runtime image.
- `deploy/docker-compose.yml` still depends on `./backend.toml:/etc/chronicle-engine-backend/backend.toml:ro`.
- `backend/src/config.rs` validates the parsed TOML config but does not yet apply environment overrides.

## Goals / Non-goals
- Goals:
  - Bake a default `backend.toml` into the runtime image.
  - Support environment-variable overrides for backend config keys so Docker Compose can configure the deployment without mounting a TOML file by default.
  - Document the canonical override naming and update deploy examples.
- Non-goals:
  - Replacing the TOML file model entirely; file-backed config remains the base layer.
  - Adding a second config system or external secret manager.
  - Rewriting archive docs.

## Target files / modules
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/src/config.rs`
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/src/main.rs`
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/deploy/Dockerfile`
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/deploy/docker-compose.yml`
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/deploy/README.md`
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/README.md`
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/README_CN.md`
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/deploy/backend.toml.example`

## Constraints
- Keep `backend.toml` as the canonical schema and validation source.
- Environment overrides must be deterministic and documented.
- Preserve current backend verification gates.
- Do not require a bind-mounted config file for the default Docker Compose path.

## Verification plan
- `cargo test --manifest-path backend/Cargo.toml --test contract_semantics -- --nocapture`
- `cargo test --manifest-path backend/Cargo.toml --test admin_plane -- --nocapture`
- `cargo clippy --manifest-path backend/Cargo.toml --all-targets --all-features -- -D warnings`
- `docker compose -f deploy/docker-compose.yml config`
- `git diff --check`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/backend-env-config-2026-03-29`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/backend-env-config-2026-03-29 README.md`

## Rollback
- Restore the compose bind mount for `deploy/backend.toml`.
- Remove env override loading from backend config.
- Remove the baked config copy from the runtime image.

## Open questions
- None. Use `CHRONICLE_`-prefixed environment overrides over the baked TOML base.
