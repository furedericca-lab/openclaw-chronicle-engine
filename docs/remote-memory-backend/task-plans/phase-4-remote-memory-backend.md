---
description: Verification and operator-readiness tasks for remote-memory-backend.
---

# Tasks: remote-memory-backend

## Input

- `docs/remote-memory-backend/remote-memory-backend-contracts.md`
- `docs/remote-memory-backend/technical-documentation.md`
- phases 2-3 implementation results
- local shell tests and backend contract tests

## Canonical architecture / Key constraints

- No mixed-authority rollback shortcuts.
- Recall remains fail-open.
- Explicit write/delete failures remain surfaced.
- Reflection enqueue remains non-blocking.
- Admin endpoints may bypass ACL only with admin token.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 4: Verification, migration safety, and ops readiness
Goal: validate behavior, migration safety, and operator-facing guardrails for the remote backend split.
Definition of Done: shell and backend tests prove the new authority model, failure behavior, and async reflection semantics are working and documented.

Tasks:
- [ ] T301 [QA] Run backend contract tests and shell behavior tests against the agreed failure semantics.
  - DoD: evidence includes passing tests for fail-open recall, surfaced write/delete errors, and non-blocking reflection enqueue.
- [ ] T302 [P] [Docs] Finalize migration and rollback notes for singular authority.
  - DoD: docs explain how to cut over fully to remote authority and how to revert fully without mixed fallback behavior.
- [ ] T303 [P] [Security] Verify admin-token bypass is isolated to explicit management routes and does not leak secrets in responses.
  - DoD: operator/admin paths are documented and test-covered with clear token-class separation.

Checkpoint: the migration is reviewable and reversible without ambiguous authority or hidden fallback paths.

## Dependencies & Execution Order

- Phase 4 depends on Phases 2-3.
- Tasks marked `[P]` may run concurrently only if they do not touch the same files.
