---
description: Phase 2 task list for distill parity migration.
---

# Tasks: distill-parity-migration-2026-03-17

## Input

- `backend/src/lib.rs`
- `backend/src/models.rs`
- `backend/src/state.rs`
- `backend/tests/phase2_contract_semantics.rs`
- `index.ts`
- `src/backend-client/client.ts`
- `src/backend-client/types.ts`

## Canonical architecture / Key constraints

- transcript rows must be persisted backend-side before `session-transcript` distill can execute;
- runtime forwarding may assist ingestion but may not become transcript authority;
- transcript cleanup/noise filtering must reuse backend-native logic instead of reviving file-batch scripts.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 2: Transcript-Source Parity

Goal: migrate transcript-source ingest/filter parity from the removed sidecar script into the Rust backend.

Definition of Done: backend-native `session-transcript` distill works, replacement tests cover migrated behavior, and the old script/direct test residue is no longer needed.

Tasks:

- [x] T101 [Backend] Implement backend-native `session-transcript` source resolution.
  - DoD: `session-transcript` jobs succeed when persisted transcript rows exist and fail with structured source-unavailable semantics only when the scoped transcript source is absent.
- [x] T102 [QA] Port transcript cleaning and noise-filter behavior into backend tests.
  - DoD: backend tests cover slash/noise filtering, injected-memory stripping, metadata cleanup, and caller-scoped transcript-source execution in `backend/tests/phase2_contract_semantics.rs`.
- [x] T103 [Agentic] Forward runtime transcript rows into the backend-owned transcript source.
  - DoD: `index.ts` appends transcript rows at `agent_end` through `POST /v1/session-transcripts/append` with deterministic idempotency.
- [x] T104 [Docs] Remove script/direct-test residue after replacement coverage exists.
  - DoD: `scripts/jsonl_distill.py` and `test/jsonl-distill-slash-filter.test.mjs` are removed and active docs no longer describe transcript parity as open.

Checkpoint: backend-native transcript-source parity is closed and the repo no longer depends on the old script for active behavior.

## Dependencies & Execution Order

- Phase 2 depended on Phase 1.
- T101 blocked T104.
- T102 and T103 could proceed once the backend transcript contract shape was frozen.
