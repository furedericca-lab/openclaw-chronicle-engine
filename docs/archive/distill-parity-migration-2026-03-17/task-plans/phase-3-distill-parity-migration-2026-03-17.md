---
description: Phase 3 task list for distill parity migration.
---

# Tasks: distill-parity-migration-2026-03-17

## Input

- `backend/src/state.rs`
- `backend/tests/phase2_contract_semantics.rs`
- `README.md`
- `README_CN.md`
- `docs/remote-memory-backend-2026-03-17/*`

## Canonical architecture / Key constraints

- reducer parity must stay backend-native and deterministic;
- sidecar topology must not return as an alternate supported architecture;
- cleanup must leave canonical docs aligned with the shipped runtime.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 3: Reducer Parity And Cleanup

Goal: replace the old example worker's useful reduction behavior with backend-native parity and remove the obsolete sidecar example.

Definition of Done: backend artifacts no longer depend on the example worker as a live reference, and active docs present only the backend-native runtime.

Tasks:

- [x] T201 [Backend] Implement backend-native candidate reduction parity.
  - DoD: backend reduction covers duplicate suppression, evidence gating, vague-advice filtering, transcript windowing, and stable artifact shaping in `backend/src/state.rs`.
- [x] T202 [QA] Add reducer/artifact-quality tests.
  - DoD: backend tests prove transcript-source completion and deterministic reduction behavior without relying on the removed example worker.
- [x] T203 [Docs] Remove old example hook/worker/systemd residue.
  - DoD: `examples/new-session-distill/*` is removed from the active repo runtime.
- [x] T204 [Docs] Refresh README and remote backend docs to the post-cleanup state.
  - DoD: active docs no longer present the old sidecar example as an active migration dependency or describe `session-transcript` as deferred.

Checkpoint: the repo contains only the backend-native distill path as active guidance and implementation.

## Dependencies & Execution Order

- Phase 3 depended on Phases 1-2.
- T201 blocked T203 because residue removal required shipped reducer parity.
- T204 closed the scope only after T201-T203 and verification passed.
