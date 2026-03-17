---
description: Task list for distill-iteration-runtime-2026-03-18 phase 2.
---

# Tasks: distill-iteration-runtime-2026-03-18 Phase 2

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

## Phase 2: Backend Deterministic Distill Upgrade
Goal: Replace message-only distill reduction with deterministic span/window synthesis, stronger heuristics, and multi-message evidence aggregation.

Definition of Done: All phase tasks are implemented, tested, and evidenced with commands and outputs.

Tasks:
- [x] T021 [Backend] Implement deterministic span/window candidate synthesis in [backend/src/state.rs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/src/state.rs).
  - DoD: reducer no longer operates only on single-message truncation and can emit multi-message evidence-backed artifacts.
- [x] T022 [P] [QA] Add backend contract coverage in [backend/tests/phase2_contract_semantics.rs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/tests/phase2_contract_semantics.rs).
  - DoD: tests cover structured English summary output and aggregated evidence across multiple messages.
- [x] T023 [Security] Preserve backend-only transcript authority while improving reduction.
  - DoD: implementation does not add local file reads, sidecar paths, or model-backed extraction.

Checkpoint: Backend distill quality improvements are implemented and passing targeted cargo tests.

## Dependencies & Execution Order
- Phase 1 blocks all others.
- Phase 2 depends on completion of phases 1-1.
- Tasks marked [P] within this phase may run concurrently only when they do not touch the same files.
- If this phase archives/removes residue, document the cleanup gate before deletion.
