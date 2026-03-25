---
description: API and schema contracts for backend-dependency-upgrades-2026-03-25.
---

# backend-dependency-upgrades-2026-03-25 Contracts

## API Contracts
- The public backend HTTP surface must remain stable:
  - `POST /v1/recall/generic`
  - `POST /v1/recall/behavioral`
  - `POST /v1/debug/recall/generic`
  - `POST /v1/debug/recall/behavioral`
  - memory management, transcript append, and distill job routes
- No dependency-upgrade batch may reintroduce public `reflection` aliases or local-authority fallback behavior.

## Shared Types / Schemas
- `backend/src/models.rs` remains the DTO source of truth during this scope.
- `contract_semantics` remains the frozen backend contract test target.
- Storage schema and behavioral naming stay unchanged unless a later phase documents an unavoidable compatibility migration.

## Event and Streaming Contracts
- No new streaming/event surfaces are introduced by this scope.
- Distill remains an async job model with the current enqueue/status routes.

## Error Model
- Internal dependency fallout may change implementation details, but external status/code behavior should remain stable.
- Fail-open/fail-closed behavior at the plugin/backend boundary must not weaken.

## Validation and Compatibility Rules
- Phase 1 records the result that the low-risk semver-compatible set is already locked at the latest compatible versions in `Cargo.lock`.
- Higher-risk upgrades must land in auditable batches:
  - Phase 2: `clap`, `http`, `lancedb`
  - Phase 3: `axum`, `reqwest`, `rusqlite`, `toml`
  - Phase 4: `arrow-array`, `arrow-schema`
- Every implementation batch must pass:
  - `cargo clippy --manifest-path backend/Cargo.toml --all-targets --all-features -- -D warnings`
  - `cargo test --manifest-path backend/Cargo.toml --test contract_semantics -- --nocapture`
  - `npm test`
