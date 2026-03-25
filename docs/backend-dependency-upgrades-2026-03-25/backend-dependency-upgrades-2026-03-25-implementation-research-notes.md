---
description: Implementation research notes for backend-dependency-upgrades-2026-03-25.
---

# backend-dependency-upgrades-2026-03-25 Implementation Research Notes

## Baseline (Current State)
- The repo contains one Rust manifest: `backend/Cargo.toml`.
- Safe-group audit on 2026-03-25 showed the following crates are already locked at latest compatible versions in `backend/Cargo.lock`:
  - `anyhow 1.0.102`
  - `futures 0.3.32`
  - `parking_lot 0.12.5`
  - `regex 1.12.3`
  - `serde 1.0.228`
  - `serde_json 1.0.149`
  - `tokio 1.50.0`
  - `uuid 1.22.0`
  - `tempfile 3.27.0`
  - `tower 0.5.3`
- `cargo update` for that safe set returned `Locking 0 packages to latest compatible versions`.

## Gap Analysis
- `clap`, `http`, and `lancedb` are behind current stable releases but remain within a bounded compatibility surface.
- `axum`, `reqwest`, `rusqlite`, and `toml` may require API or behavior changes across request handling, transport, SQLite persistence, and config parsing.
- `arrow-array` and `arrow-schema` are tightly coupled to Lance/Arrow ecosystem compatibility and should be isolated last.

## Candidate Designs and Trade-offs
- Option A: upgrade everything in one pass.
  - Rejected because breakage becomes impossible to localize.
- Option B: batch by ecosystem/runtime boundary.
  - Selected because it isolates medium-risk CLI/protocol/storage upgrades from framework/transport/config and Arrow/Lance compatibility work.

## Selected Design
- Phase 1: record the audit and no-op result for the safe set.
- Phase 2: upgrade `clap`, `http`, and `lancedb`.
- Phase 3: upgrade `axum`, `reqwest`, `rusqlite`, and `toml`.
- Phase 4: upgrade `arrow-array` and `arrow-schema` and absorb any compatibility fallout.

## Validation Plan
- For every implementation batch:
  - `cargo clippy --manifest-path backend/Cargo.toml --all-targets --all-features -- -D warnings`
  - `cargo test --manifest-path backend/Cargo.toml --test contract_semantics -- --nocapture`
  - `npm test`
- For deployment-sensitive fallout:
  - `docker compose -f deploy/docker-compose.yml config`

## Risks and Assumptions
- `lancedb` may implicitly constrain acceptable Arrow/Lance versions.
- `axum` and `reqwest` may require helper rewrites in handlers or tests.
- `rusqlite` and `toml` may change behavior even when compilation succeeds.
