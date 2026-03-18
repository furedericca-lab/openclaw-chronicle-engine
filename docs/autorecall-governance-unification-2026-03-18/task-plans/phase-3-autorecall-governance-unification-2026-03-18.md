---
description: Task list for autorecall-governance-unification-2026-03-18 phase 3.
---

# Tasks: autorecall-governance-unification-2026-03-18 Phase 3

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

## Phase 3: Governance Surface Migration
Goal: Remove self-improvement naming from active tooling/docs/tests and align reminder/bootstrap behavior with behavioral autoRecall guidance.

Definition of Done: All phase tasks are implemented, tested, and evidenced with commands and outputs.

Tasks:
- [ ] T041 [Backend] Re-home governance backlog tools and files.
  - DoD: `src/governance-tools.ts` or equivalent canonical module owns log/review/extract behavior and governance backlog file initialization; active runtime imports use that module.
- [ ] T042 [P] [Docs] Update active docs/tests/config strings to the new architecture.
  - DoD: `README.md`, `README_CN.md`, `docs/runtime-architecture.md`, `docs/context-engine-split-2026-03-17/*`, `openclaw.plugin.json`, and targeted `test/` files use autoRecall behavioral guidance + governance terminology; any remaining legacy terms are explicitly marked transitional.
- [ ] T043 [Security] Validate governance path migration and alias safety.
  - DoD: governance extraction output remains sanitized under workspace, legacy file/config aliases are bounded and documented, and no new write path escapes are introduced.

Checkpoint: Phase 3 artifacts are merged, verified, and recorded in 4phases-checklist.md before next phase starts.

## Dependencies & Execution Order
- Phase 1 blocks all others.
- Phase 3 depends on completion of Phases 1-2.
- Tasks marked [P] within this phase may run concurrently only when they do not touch the same files.
