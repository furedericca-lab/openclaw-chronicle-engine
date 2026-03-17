---
description: Task list for distill-iteration-runtime-2026-03-18 phase 3.
---

# Tasks: distill-iteration-runtime-2026-03-18 Phase 3

## Input
- Canonical sources:
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/README.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/distill-iteration-runtime-2026-03-18/distill-iteration-runtime-2026-03-18-scope-milestones.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/distill-iteration-runtime-2026-03-18/distill-iteration-runtime-2026-03-18-technical-documentation.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/distill-iteration-runtime-2026-03-18/distill-iteration-runtime-2026-03-18-contracts.md

## Canonical architecture / Key constraints
- Keep architecture aligned with distill-iteration-runtime-2026-03-18 scope docs and contracts.
- Keep provider/runtime/channel boundaries unchanged unless explicitly in scope.
- Keep security and test gates in Definition of Done.
- If parity or migration is in scope, preserve required behavior without assuming historical implementation shape must be recreated.

## Format
- [ID] [P?] [Component] Description
- [P] means parallelizable.
- Valid components: Backend, Frontend, Agentic, Docs, Config, QA, Security, Infra.
- Every task must have a clear DoD.

## Phase 3: Runtime Cadence, Config, And Docs
Goal: Add cadence-based automatic distill enqueue at `agent_end`, expose config/schema, and refresh tests/docs.

Definition of Done: All phase tasks are implemented, tested, and evidenced with commands and outputs.

Tasks:
- [x] T041 [Config] Add `distill` runtime config parsing and schema in [index.ts](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/index.ts) and [openclaw.plugin.json](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/openclaw.plugin.json).
  - DoD: runtime supports optional automatic backend distill cadence every N user turns.
- [x] T042 [P] [QA] Add shell integration coverage in [test/remote-backend-shell-integration.test.mjs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/test/remote-backend-shell-integration.test.mjs).
  - DoD: tests prove config parsing, assistant-only non-advancement, and automatic enqueue on cadence boundary.
- [x] T043 [Docs] Refresh README and scope docs for the new deterministic English-only distill behavior.
  - DoD: active docs describe automatic cadence and backend-native deterministic distill correctly.

Checkpoint: Runtime cadence, docs, and verification are complete.

## Dependencies & Execution Order
- Phase 1 blocks all others.
- Phase 3 depends on completion of phases 1-2.
- Tasks marked [P] within this phase may run concurrently only when they do not touch the same files.
- If this phase archives/removes residue, document the cleanup gate before deletion.
