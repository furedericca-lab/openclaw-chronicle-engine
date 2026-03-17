---
description: Phase 1 task list for distill parity migration.
---

# Tasks: distill-parity-migration-2026-03-17

## Input

- `docs/archive/distill-backend-scope/*`
- `docs/remote-memory-backend-2026-03-17/*`
- `docs/archive/distill-parity-migration-2026-03-17/*`

## Canonical architecture / Key constraints

- Rust backend owns transcript-source authority, job execution, and artifact persistence.
- Acceptable parity is behavioral, not a reintroduction of sidecar topology.
- Cleanup is gated by shipped backend replacement coverage.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 1: Contract Freeze

Goal: freeze acceptable parity rules for transcript-source and reducer migration.

Definition of Done: the scope clearly distinguishes backend-owned parity from rejected historical sidecar shapes, and every residue path has a cleanup gate.

Tasks:

- [x] T001 [Docs] Freeze the remaining parity buckets and acceptable Rust replacements.
  - DoD: research notes and technical docs distinguish required behavioral parity from rejected historical shape recreation.
- [x] T002 [Security] Freeze the authority boundary for transcript-source resolution and persistence.
  - DoD: contracts explicitly reject local sidecar authority patterns from re-entering the supported runtime.
- [x] T003 [Docs] Freeze cleanup gates for script, direct test, and example worker residue.
  - DoD: cleanup readiness matrix is documented and unambiguous.

Checkpoint: implementation may proceed without reopening the local-vs-remote ownership question.

## Dependencies & Execution Order

- Phase 1 blocked all later phases.
- Phase 2 depended on the frozen authority boundary and cleanup gates.
- Phase 3 depended on Phase 2 shipping backend transcript parity first.
