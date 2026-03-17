---
description: Execution and verification checklist for ts-residual-debt-cleanup-2026-03-17 3-phase plan.
---

# Phases Checklist: ts-residual-debt-cleanup-2026-03-17

## Input

- Canonical docs under:
  - `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/ts-residual-debt-cleanup-2026-03-17`
  - `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/ts-residual-debt-cleanup-2026-03-17/task-plans`

## Rules

- use this file as the single progress and audit hub;
- update status, evidence commands, and blockers after each implementation batch;
- do not mark a phase complete without evidence.

## Global Status Board

- Phase 1: completed, 100%, healthy, blockers: none
- Phase 2: completed, 100%, healthy, blockers: none
- Phase 3: completed, 100%, healthy, blockers: none

## Phase Entry Links

1. [phase-1-ts-residual-debt-cleanup-2026-03-17.md](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/ts-residual-debt-cleanup-2026-03-17/task-plans/phase-1-ts-residual-debt-cleanup-2026-03-17.md)
2. [phase-2-ts-residual-debt-cleanup-2026-03-17.md](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/ts-residual-debt-cleanup-2026-03-17/task-plans/phase-2-ts-residual-debt-cleanup-2026-03-17.md)
3. [phase-3-ts-residual-debt-cleanup-2026-03-17.md](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/ts-residual-debt-cleanup-2026-03-17/task-plans/phase-3-ts-residual-debt-cleanup-2026-03-17.md)

## Phase Execution Records

### Phase 1

- Completion checklist:
  - [x] target helper set frozen
  - [x] import/use-site evidence recorded
  - [x] file-by-file classification documented
- Evidence commands + result status:
  - `rg -n "recall-engine|auto-recall-final-selection|final-topk-setwise-selection|reflection-recall|reflection-recall-final-selection|adaptive-retrieval" src test index.ts`
    - pass: import/use-site evidence collected for the pre-cleanup baseline
  - source inspection completed for:
    - `src/recall-engine.ts`
    - `src/adaptive-retrieval.ts`
    - `src/auto-recall-final-selection.ts`
    - `src/final-topk-setwise-selection.ts`
    - `src/reflection-recall.ts`
    - `src/reflection-recall-final-selection.ts`
- Issues / blockers and resolutions:
  - no blocker in the audit phase; the main clarification was that the remaining debt was naming/location debt rather than surviving authority debt
- Checkpoint confirmation:
  - Phase 1 complete; cleanup can target specific files without reopening the authority-parity question.

### Phase 2

- Completion checklist:
  - [x] test/reference helper movement plan executed
  - [x] affected tests updated
  - [x] no production imports remain
- Evidence commands + result status:
  - `rg -n "reflection-recall-reference|reflection-recall-selection-reference|reflection-recall\\.ts|reflection-recall-final-selection" src test index.ts`
    - pass: relocated helper imports limited to `test/helpers/*`; no production imports remain
  - `node --test --test-name-pattern='.' test/memory-reflection.test.mjs`
    - pass
- Issues / blockers and resolutions:
  - none recorded
- Checkpoint confirmation:
  - completed: test/reference helper residue no longer occupies ambiguous top-level runtime space.

### Phase 3

- Completion checklist:
  - [x] retained prompt-local helpers renamed/split/annotated
  - [x] docs updated to final file layout
  - [x] plugin test suite passes after cleanup
- Evidence commands + result status:
  - `rg -n "prompt-local-auto-recall-selection|prompt-local-topk-setwise-selection|auto-recall-final-selection|final-topk-setwise-selection" src test README.md README_CN.md docs/archive/ts-residual-debt-cleanup-2026-03-17`
    - pass: runtime/test/docs point at the prompt-local names, with old-name mentions preserved only in historical Phase 1 audit records
  - `npm test`
    - pass: `93 passed / 0 failed`
- Issues / blockers and resolutions:
  - none recorded
- Checkpoint confirmation:
  - implementation complete pending final verification batch.

## Final Release Gate

- Scope constraints preserved.
- Quality/security gates passed: yes
- Remaining risks documented: yes
