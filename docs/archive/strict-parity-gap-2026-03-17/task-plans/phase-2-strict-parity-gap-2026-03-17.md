---
description: Close backend observability and retrieval traceability gaps under the strict parity definition.
---

# Tasks: strict-parity-gap-2026-03-17

## Input

- `docs/archive/strict-parity-gap-2026-03-17/strict-parity-gap-2026-03-17-technical-documentation.md`
- `docs/archive/strict-parity-gap-2026-03-17/strict-parity-gap-2026-03-17-contracts.md`
- `docs/archive/strict-parity-gap-2026-03-17/strict-parity-gap-2026-03-17-implementation-research-notes.md`
- `backend/src/state.rs`
- `backend/src/models.rs`
- `backend/src/main.rs`
- `backend/tests/phase2_contract_semantics.rs`

## Canonical architecture / Key constraints

- backend remains authoritative for retrieval behavior and traceability;
- no internal trace fields may leak into normal `/v1/recall/*` rows;
- any admin/debug surface must be additive and explicitly authorized;
- current retrieval behavior must not regress while observability is strengthened.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 2: Backend Trace Parity

Goal: upgrade current internal diagnostics into a stricter retrieval traceability surface that better matches historical TS debugging power or an explicitly accepted Rust-native equivalent.

Definition of Done: backend exposes a stable internal/admin/debug retrieval trace model or equivalent inspection mechanism, tests prove DTO non-leakage, and docs define how operators/tests can inspect trace behavior.

Tasks:

- [x] T101 [Backend] Introduce a structured retrieval trace model beyond event-only diagnostics.
  - DoD: `backend/src/state.rs` and any extracted backend modules record per-stage retrieval decisions, candidate/result counts, fallback reasons, and finalization outcomes in a stable schema.
- [x] T102 [Backend] Add an internal or admin/debug retrieval trace inspection path.
  - DoD: implementation provides a documented inspection mechanism for traces using real backend paths/modules or stable test-visible hooks, with authorization semantics captured in docs/contracts when externally exposed.
- [x] T103 [P] [QA] Add backend regression tests for trace visibility and DTO non-leakage.
  - DoD: `backend/tests/phase2_contract_semantics.rs` or split backend tests assert trace availability, authorization rules, and absence of internal fields in `/v1` runtime DTOs.
- [x] T104 [Security] Validate redaction and principal-boundary rules for trace surfaces.
  - DoD: tests/docs confirm sensitive fields are bounded/redacted and that cross-principal trace exposure is impossible without explicit admin authorization.
- [x] T105 [Docs] Record exact trace surface behavior and verification commands.
  - DoD: `strict-parity-gap-2026-03-17-technical-documentation.md`, `...-contracts.md`, and the checklist specify commands and expected outcomes for trace inspection and leak-prevention verification.

Checkpoint: backend retrieval traceability is no longer weaker than the old TS system in ambiguous or ad hoc ways, even if the Rust implementation shape differs.

## Dependencies & Execution Order

- Phase 2 depends on Phase 1.
- `T101` blocks `T102` and `T103`.
- `T104` depends on the candidate trace surface from `T101`/`T102`.
- `T105` closes the phase after implementation and tests land.

## Execution Record

### Implemented

- added backend-owned retrieval trace payloads for generic and reflection recall flows in `backend/src/state.rs`;
- added explicit debug trace routes in `backend/src/lib.rs`:
  - `POST /v1/debug/recall/generic`
  - `POST /v1/debug/recall/reflection`
- kept ordinary `/v1/recall/*` DTO rows unchanged while returning trace only from debug-scoped responses;
- recorded rerank fallback reason, seed-stage outcomes, final row ids, and access-update status in trace output;
- added backend tests covering trace visibility, principal-boundary enforcement, rerank fallback trace capture, reflection-mode trace capture, and DTO non-leakage.

### Evidence

- backend tests:
  - `cargo test --manifest-path /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
- key files:
  - `backend/src/lib.rs`
  - `backend/src/models.rs`
  - `backend/src/state.rs`
  - `backend/tests/phase2_contract_semantics.rs`

### Phase 2 checkpoint result

- completed: backend trace parity now uses explicit debug-scoped routes with stable structured traces and no leakage into ordinary runtime recall DTOs.
