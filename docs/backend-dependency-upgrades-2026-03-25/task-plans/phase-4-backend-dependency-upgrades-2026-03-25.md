---
description: Task list for backend-dependency-upgrades-2026-03-25 phase 4.
---

# Tasks: backend-dependency-upgrades-2026-03-25 Phase 4

## Input
- Canonical sources:
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/README.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/backend-dependency-upgrades-2026-03-25/backend-dependency-upgrades-2026-03-25-scope-milestones.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/backend-dependency-upgrades-2026-03-25/backend-dependency-upgrades-2026-03-25-technical-documentation.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/backend-dependency-upgrades-2026-03-25/backend-dependency-upgrades-2026-03-25-contracts.md

## Canonical architecture / Key constraints
- Keep architecture aligned with backend-dependency-upgrades-2026-03-25 scope docs and contracts.
- Keep provider/runtime/channel boundaries unchanged unless explicitly in scope.
- Keep security and test gates in Definition of Done.

## Format
- [ID] [P?] [Component] Description
- [P] means parallelizable.
- Valid components: Backend, Frontend, Agentic, Docs, Config, QA, Security, Infra.
- Every task must have a clear DoD.

## Phase 4: Arrow Compatibility Closeout
Goal: Upgrade `arrow-array` and `arrow-schema` and absorb any Lance/Arrow compatibility fallout.

Definition of Done: All phase tasks are implemented, tested, and evidenced with commands and outputs.

Tasks:
- [ ] T061 [Backend] Upgrade `arrow-array` and `arrow-schema`.
  - DoD: schema builders and seeded fixtures compile and behave correctly.
- [ ] T062 [Backend] Resolve any required compatibility fallout between Arrow and `lancedb`/`lance-*`.
  - DoD: storage/index/query behavior remains stable under contract tests.
- [ ] T063 [QA] Run full release-line verification and record residual deferrals if any remain.
  - DoD: `clippy`, `contract_semantics`, `npm test`, and deploy config checks are recorded.

Checkpoint: Phase 4 artifacts are merged, verified, and recorded in 4phases-checklist.md before next phase starts.

## Dependencies & Execution Order
- Phase 1 blocks all others.
- Phase 4 depends on completion of phases 1-3.
- Tasks marked [P] within this phase may run concurrently only when they do not touch the same files.
