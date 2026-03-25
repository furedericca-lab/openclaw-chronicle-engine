---
description: Brainstorming and decision framing for backend-dependency-upgrades-2026-03-25.
---

# backend-dependency-upgrades-2026-03-25 Brainstorming

## Problem
- The user requested a dependency freshness pass for the Rust backend and wants to start from the safest upgrade set first.
- The safe semver-compatible set already resolves to the latest compatible versions in `backend/Cargo.lock`, so forcing churn there would add no value.
- The remaining crates touch different backend boundaries and should not be upgraded as one opaque batch.

## Scope
- Plan the risky backend dependency upgrades:
  - `clap`, `http`, `lancedb`
  - `axum`, `reqwest`, `rusqlite`, `toml`
  - `arrow-array`, `arrow-schema`
- Keep the public backend contract stable while doing so.

## Constraints
- `contract_semantics` must remain the backend contract test gate.
- Release-line hygiene must stay intact: `clippy -D warnings`, backend tests, and `npm test` must continue to pass.
- No backend crate or binary rename is allowed in this scope.

## Options
- Option A: upgrade all remaining crates in one batch.
  - Rejected because failures in framework, transport, SQLite, config parsing, LanceDB, and Arrow would be hard to localize.
- Option B: batch by runtime boundary and ecosystem coupling.
  - Selected because it minimizes the blast radius of each step and keeps rollback simple.

## Decision
- Treat the safe set as Phase 1 audit evidence, not as a forced code-change batch.
- Run the remaining upgrades in phased order:
  - Phase 2: `clap`, `http`, `lancedb`
  - Phase 3: `axum`, `reqwest`, `rusqlite`, `toml`
  - Phase 4: `arrow-array`, `arrow-schema`

## Risks
- `lancedb` may implicitly constrain acceptable Arrow/Lance versions.
- `axum` and `reqwest` may require handler/provider helper rewrites.
- `rusqlite` and `toml` may change runtime behavior without obvious compile errors.

## Open Questions
- Whether `lancedb` can move cleanly before Arrow major alignment, or whether part of Phase 2 must defer into Phase 4.
