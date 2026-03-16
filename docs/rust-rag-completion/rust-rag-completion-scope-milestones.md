---
description: Scope boundaries and milestones for completing the Rust-owned RAG pipeline.
---

# rust-rag-completion Scope and Milestones

## In Scope
- Real generic recall implementation in Rust for `backend/`
- Embedding provider wiring in Rust backend config/runtime
- Candidate generation, hybrid retrieval, ranking, and deterministic top-k selection in Rust
- Reflection-recall path hardening when directly affected by shared retrieval components
- Backend tests proving retrieval is no longer placeholder-only
- Docs and verification evidence under `docs/rust-rag-completion/`

## Out of Scope
- Reintroducing the deleted local TypeScript RAG authority
- Multi-node backend workers or queue infra
- UI / frontend changes
- Unrelated shell/runtime refactors
- Broad operator admin API surface expansion

## Milestones
### Milestone 1 — Retrieval source mapping and design freeze
- Identify the authoritative old TS retrieval behaviors worth preserving.
- Decide the Rust implementation path for embedding, candidate generation, hybrid merge, rerank, and deterministic selection.
- Freeze what is deliberately deferred.

### Milestone 2 — Generic RAG implementation in Rust
- Replace placeholder generic recall logic in `backend/src/state.rs`.
- Load provider/config inputs in Rust.
- Land real candidate generation and ranking.
- Keep runtime DTO shape stable.

### Milestone 3 — Reflection/shared-path hardening
- Ensure reflection recall is not silently left on placeholder-only semantics when shared retrieval code changes.
- Add any missing tests for mode semantics and ranking behavior.
- Align docs with the actual backend capability.

### Milestone 4 — Verification and closeout
- Run formatting, build, and tests.
- Record evidence and residual risks.
- Leave the verify worktree in a reviewable state for final acceptance.

## Dependencies
- Existing backend contract routes and auth middleware in `backend/`
- Existing TypeScript retrieval logic for behavioral reference
- Backend provider configuration design in current docs / code
- LanceDB capabilities available through the current Rust dependency set

## Exit Criteria
- Generic recall is demonstrably no longer placeholder scoring.
- The backend compiles and tests pass.
- Docs record what was implemented, verified, and still deferred.
- No local TS authority is reintroduced.
