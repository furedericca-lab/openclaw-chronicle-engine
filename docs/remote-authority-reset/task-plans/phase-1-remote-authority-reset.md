---
description: Documentation reset and canonical architecture freeze for remote-authority-reset.
---

# Tasks: remote-authority-reset

## Input
- `README.md`
- `README_CN.md`
- `docs/archive/README.md`
- `docs/context-engine-split/*` (historical source set before archival)
- `docs/remote-memory-backend/*` (historical source set before archival)
- `docs/remote-authority-reset/*`

## Canonical architecture / Key constraints
- The target architecture is singular: Rust remote authority + thin OpenClaw adapter + local context-engine.
- Historical docs must remain preserved, but they are not canonical architecture references.
- Phase 1 is documentation-only and must not invent extra product scope.

## Format
- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 1: Documentation reset and architecture freeze
Goal: Archive the old active architecture docs and establish one new canonical architecture scope.
Definition of Done: Archive paths exist, canonical docs exist, and user-facing top-level references point to the new scope.

Tasks:
- [ ] T001 [Docs] Archive the previous active scope roots.
  - DoD: `docs/context-engine-split/` and `docs/remote-memory-backend/` are preserved under `docs/archive/2026-03-15-architecture-reset/` with no content loss.
- [ ] T002 [P] [Docs] Create the new canonical architecture scope under `docs/remote-authority-reset/`.
  - DoD: canonical README, contracts, technical docs, implementation notes, milestones, and phased plans exist and describe the new target architecture in concrete repo terms.
- [ ] T003 [P] [Docs] Update top-level references to canonical docs.
  - DoD: `README.md`, `README_CN.md`, and `docs/archive/README.md` point readers to `docs/remote-authority-reset/` and clearly describe the archived status of the old scopes.
- [ ] T004 [QA] Run documentation hygiene checks for the new canonical scope.
  - DoD: placeholder scan, residual scan, and `git diff --check` pass or any failures are documented with exact follow-up actions.

Checkpoint: Future work can rely on `docs/remote-authority-reset/` as the only canonical architecture source.

## Dependencies & Execution Order
- Phase 1 blocks all others.
- `T001` must complete before finalizing `T003` because README/archive references depend on the final archive path.
- `T002` and `T003` may run in parallel after the archive path is fixed.
