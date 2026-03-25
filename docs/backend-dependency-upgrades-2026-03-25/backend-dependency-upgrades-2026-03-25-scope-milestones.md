---
description: Scope boundaries and milestones for backend-dependency-upgrades-2026-03-25.
---

# backend-dependency-upgrades-2026-03-25 Scope and Milestones

## In Scope
- Audit current backend Rust dependencies against crates.io stable releases.
- Record that the low-risk semver-compatible set already resolves to the latest compatible versions in `backend/Cargo.lock`:
  - `anyhow`, `futures`, `parking_lot`, `regex`, `serde`, `serde_json`, `tokio`, `uuid`, `tempfile`, `tower`
- Plan and execute higher-risk dependency upgrades in bounded batches:
  - `clap`, `http`, `lancedb`
  - `axum`, `reqwest`, `rusqlite`, `toml`
  - `arrow-array`, `arrow-schema`

## Out of Scope
- TypeScript/npm dependency upgrades.
- Backend crate/binary renaming.
- Storage schema migrations unrelated to dependency compatibility.
- Runtime contract expansion unrelated to dependency fallout.

## Milestones
- Milestone 1: complete the dependency audit and record the no-op result for the safe set.
- Milestone 2: upgrade `clap`, `http`, and `lancedb`.
- Milestone 3: upgrade `axum`, `reqwest`, `rusqlite`, and `toml`.
- Milestone 4: upgrade `arrow-array` and `arrow-schema`, then close with full verification.

## Dependencies
- `backend/Cargo.toml`
- `backend/Cargo.lock`
- `backend/src/lib.rs`
- `backend/src/models.rs`
- `backend/src/state.rs`
- `backend/tests/contract_semantics.rs`
- `deploy/Dockerfile`

## Exit Criteria
- All planned upgrade groups either land successfully or are explicitly deferred with documented blockers.
- Backend contract behavior remains stable and test-backed.
- Deployment docs and Docker build remain compatible with the resulting dependency set.
