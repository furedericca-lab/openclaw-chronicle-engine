---
description: Behavior-parity verification and documentation updates for context-engine-split.
---

# Tasks: context-engine-split

## Input
- `docs/context-engine-split/technical-documentation.md`
- `README.md`
- `README_CN.md`
- `test/memory-reflection.test.mjs`
- `test/config-session-strategy-migration.test.mjs`
- phase-2 implementation diff

## Canonical architecture / Key constraints
- Do not overclaim a shipped ContextEngine migration.
- Documentation must distinguish current framework contract from future architecture direction.
- Active-path verification evidence is mandatory.

## Format
- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 3: Behavior parity and docs alignment
Goal: prove that extraction preserved behavior and align repo docs with the new internal boundary.
Definition of Done: test suite evidence is recorded and user-facing docs describe the internal split accurately.

Tasks:
- [ ] T201 [QA] Run regression tests for config, reflection, and self-improvement active paths.
  - DoD: `npm test` plus focused test commands are executed; pass/fail results are recorded in `4phases-checklist.md`.
- [ ] T202 [P] [Docs] Update README / README_CN architecture and module-boundary sections.
  - DoD: docs explain backend-vs-orchestration separation without claiming the plugin already ships as a ContextEngine.
- [ ] T203 [Security] Review hook-path parity and failure-mode logging after extraction.
  - DoD: logs/fail-open behavior for orchestrator errors remain explicit and documented.

Checkpoint: docs and tests agree on the same current-state contract and validated boundary split.

## Dependencies & Execution Order
- Phase 3 depends on Phase 2.
- T202 may run in parallel with T201 only if implementation is already stabilized.
