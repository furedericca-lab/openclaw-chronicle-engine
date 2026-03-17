---
description: Reduce naming and seam ambiguity in retained prompt-local TS helpers.
---

# Tasks: ts-residual-debt-cleanup-2026-03-17

## Input

- `docs/archive/ts-residual-debt-cleanup-2026-03-17/ts-residual-debt-cleanup-2026-03-17-implementation-research-notes.md`
- `docs/archive/ts-residual-debt-cleanup-2026-03-17/ts-residual-debt-cleanup-2026-03-17-technical-documentation.md`
- `docs/archive/ts-residual-debt-cleanup-2026-03-17/ts-residual-debt-cleanup-2026-03-17-contracts.md`
- `src/recall-engine.ts`
- `src/adaptive-retrieval.ts`
- `src/prompt-local-auto-recall-selection.ts`
- `src/prompt-local-topk-setwise-selection.ts`
- `src/context/*`
- `README.md`
- `README_CN.md`

## Canonical architecture / Key constraints

- retained production helpers may remain local when they are prompt-local only;
- this phase improves clarity, not backend authority ownership;
- README and runtime docs must stay consistent with the final retained helper layout.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 3: Naming and Seam Clarity

Goal: make retained production TS helpers visibly prompt-local so they no longer read like unfinished backend migration residue.

Definition of Done: the retained production helper set has clearer naming/location and README/docs stay consistent with the final boundary.

Tasks:

- [x] T201 [Frontend] Rename retained production helpers with the highest authority-sounding debt.
  - DoD: the cleanup explicitly addresses `src/auto-recall-final-selection.ts` and `src/final-topk-setwise-selection.ts` without changing backend authority behavior.
- [x] T202 [P] [Docs] Update README and scope docs to match the final retained helper layout.
  - DoD: docs no longer describe the old names/locations as if they were current.
- [x] T203 [QA] Re-run plugin test suites and import scans after naming cleanup.
  - DoD: `npm test` and focused import scans pass or any failures are documented with unblock plan.

Checkpoint: remaining TS runtime-local code reads as prompt-local orchestration, not latent backend authority residue.

## Dependencies & Execution Order

- Phase 3 depends on the classification from Phase 1 and accounts for the path movement from Phase 2.

## Execution Record

### Implemented

- renamed `src/auto-recall-final-selection.ts` to `src/prompt-local-auto-recall-selection.ts`;
- renamed `src/final-topk-setwise-selection.ts` to `src/prompt-local-topk-setwise-selection.ts`;
- updated runtime imports in `src/context/auto-recall-orchestrator.ts`;
- updated tests and README/docs to reflect the final prompt-local naming.

### Evidence

- `rg -n "prompt-local-auto-recall-selection|prompt-local-topk-setwise-selection|auto-recall-final-selection|final-topk-setwise-selection" src test README.md README_CN.md docs/archive/ts-residual-debt-cleanup-2026-03-17`
- `npm test`

### Phase 3 checkpoint result

- completed: retained local helpers now read as prompt-local seams rather than unfinished backend ownership, and the plugin test suite passes after cleanup.
