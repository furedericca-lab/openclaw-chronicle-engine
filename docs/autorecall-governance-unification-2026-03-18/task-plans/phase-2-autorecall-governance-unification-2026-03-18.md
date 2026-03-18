---
description: Task list for autorecall-governance-unification-2026-03-18 phase 2.
---

# Tasks: autorecall-governance-unification-2026-03-18 Phase 2

## Input
- Canonical sources:
  - /root/verify/openclaw-chronicle-engine-autorecall-governance-unification-2026-03-18/README.md
  - /root/verify/openclaw-chronicle-engine-autorecall-governance-unification-2026-03-18/docs/autorecall-governance-unification-2026-03-18/autorecall-governance-unification-2026-03-18-scope-milestones.md
  - /root/verify/openclaw-chronicle-engine-autorecall-governance-unification-2026-03-18/docs/autorecall-governance-unification-2026-03-18/autorecall-governance-unification-2026-03-18-technical-documentation.md
  - /root/verify/openclaw-chronicle-engine-autorecall-governance-unification-2026-03-18/docs/autorecall-governance-unification-2026-03-18/autorecall-governance-unification-2026-03-18-contracts.md

## Canonical architecture / Key constraints
- Keep architecture aligned with autorecall-governance-unification-2026-03-18 scope docs and contracts.
- Keep provider/runtime/channel boundaries unchanged unless explicitly in scope.
- Keep security and test gates in Definition of Done.

## Format
- [ID] [P?] [Component] Description
- [P] means parallelizable.
- Valid components: Backend, Frontend, Agentic, Docs, Config, QA, Security, Infra.
- Every task must have a clear DoD.

## Phase 2: Runtime And Config Refactor
Goal: Make autoRecall the only prompt-time orchestration surface and split former self-improvement runtime behavior across governance and behavioral guidance.

Definition of Done: All phase tasks are implemented, tested, and evidenced with commands and outputs.

Tasks:
- [ ] T021 [Backend] Replace the dedicated reflection planner with autoRecall behavioral guidance orchestration.
  - DoD: runtime wiring in `index.ts` and `src/context/*` no longer presents reflection as a peer prompt-time architecture concept; behavioral guidance prompt blocks are emitted through the autoRecall path.
- [ ] T022 [Config] Introduce canonical config names and remove legacy config alias handling.
  - DoD: `index.ts` and `openclaw.plugin.json` ship canonical `sessionStrategy: autoRecall`, `autoRecallBehavioral`, and `governance` surfaces, and removed legacy config aliases are rejected instead of parsed.
- [ ] T023 [Security] Preserve fail-open and workspace-bound behavior during the refactor.
  - DoD: missing-principal recall remains skip-only, backend recall failures remain non-blocking, and governance file writes stay inside the resolved workspace path.

Checkpoint: Phase 2 artifacts are merged, verified, and recorded in 4phases-checklist.md before next phase starts.

## Dependencies & Execution Order
- Phase 1 blocks all others.
- Phase 2 depends on completion of Phase 1.
- Tasks marked [P] within this phase may run concurrently only when they do not touch the same files.
