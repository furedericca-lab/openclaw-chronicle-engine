---
description: Task list for adapter-surface-closeout-2026-03-17 phase 2.
---

# Tasks: adapter-surface-closeout-2026-03-17 Phase 2

## Input
- Canonical sources:
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/README.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/adapter-surface-closeout-2026-03-17/adapter-surface-closeout-2026-03-17-scope-milestones.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/adapter-surface-closeout-2026-03-17/adapter-surface-closeout-2026-03-17-technical-documentation.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/adapter-surface-closeout-2026-03-17/adapter-surface-closeout-2026-03-17-contracts.md

## Canonical architecture / Key constraints
- Keep architecture aligned with adapter-surface-closeout-2026-03-17 scope docs and contracts.
- Keep provider/runtime/channel boundaries unchanged unless explicitly in scope.
- Keep security and test gates in Definition of Done.
- New operator surfaces must consume existing backend routes, not invent new local state.
- Distill/debug surfaces should remain management-gated.

## Format
- [ID] [P?] [Component] Description
- [P] means parallelizable.
- Valid components: Backend, Frontend, Agentic, Docs, Config, QA, Security, Infra.
- Every task must have a clear DoD.

## Phase 2: Adapter Surface Completion
Goal: Expose the missing distill and recall-debug shell surfaces over existing Rust backend contracts.

Definition of Done: All phase tasks are implemented, tested, and evidenced with commands and outputs.

Tasks:
- [x] T021 [Agentic] Add typed debug-recall client methods and tool-facing DTOs.
  - DoD: `src/backend-client/types.ts` and `src/backend-client/client.ts` model and call `/v1/debug/recall/generic` and `/v1/debug/recall/reflection` without changing ordinary recall DTOs.
- [x] T022 [Agentic] Add management-gated distill and recall-debug tools.
  - DoD: `src/backend-tools.ts` registers the selected management tools for distill enqueue/status and recall debug trace behind `enableManagementTools`, and `index.ts` keeps existing ordinary tool behavior unchanged when management tools are disabled.
- [x] T023 [P] [QA] Add Node integration coverage for the new adapter surfaces.
  - DoD: `test/remote-backend-shell-integration.test.mjs` and any needed helper tests cover success, missing-principal, and backend-error cases for the new distill/debug shell surfaces.
- [x] T024 [Security] Validate gating, caller scoping, and bounded output behavior.
  - DoD: tests or explicit assertions prove the new tools require runtime principal identity, do not accept `scope`, and do not leak ordinary recall trace data through non-debug paths.

Checkpoint: Phase 2 artifacts are updated, verified, and recorded in `4phases-checklist.md` before next phase starts.

## Dependencies & Execution Order
- Phase 1 blocks all others.
- Phase 2 depends on completion of phases 1-1.
- Tasks marked [P] within this phase may run concurrently only when they do not touch the same files.
