---
description: Freeze the TS residual debt audit and file classification baseline.
---

# Tasks: ts-residual-debt-cleanup-2026-03-17

## Input

- `README.md`
- `docs/runtime-architecture.md`
- `docs/archive/strict-parity-gap-2026-03-17/*`
- `src/recall-engine.ts`
- `src/adaptive-retrieval.ts`
- `src/auto-recall-final-selection.ts`
- `src/final-topk-setwise-selection.ts`
- `src/reflection-recall.ts`
- `src/reflection-recall-final-selection.ts`
- `src/context/*`
- `test/memory-reflection.test.mjs`
- `test/remote-backend-shell-integration.test.mjs`

## Canonical architecture / Key constraints

- backend remains the only authority layer;
- local orchestration may remain local when it is clearly prompt-local;
- this phase is an audit/freeze phase, not a delete-everything phase.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 1: Audit Freeze

Goal: produce a file-by-file residual debt register with import evidence and an explicit keep/move/rename/remove classification.

Definition of Done: the scope docs clearly separate runtime prompt-local seams from test/reference helpers and identify the strongest cleanup targets without ambiguity.

Tasks:

- [x] T001 [Docs] Freeze the target helper set and current import/use-site evidence.
  - DoD: docs record concrete import evidence for `src/recall-engine.ts`, `src/adaptive-retrieval.ts`, `src/auto-recall-final-selection.ts`, `src/final-topk-setwise-selection.ts`, `src/reflection-recall.ts`, and `src/reflection-recall-final-selection.ts`.
- [x] T002 [P] [Docs] Classify each helper as prompt-local production, shared local utility, test/reference-only, or removable pending proof.
  - DoD: technical docs and research notes agree on a file-by-file classification.
- [x] T003 [Security] Freeze cleanup safety rules before any movement/deletion work.
  - DoD: contracts say production runtime must not import helpers classified as test/reference-only and ambiguous files are retained pending proof rather than deleted.

Checkpoint: engineering can start cleanup work without re-running the same dependency audit.

## Dependencies & Execution Order

- Phase 1 blocks all others.
- `T002` may run with `T001` once the helper set is fixed.
- Phase 2 and 3 should not begin until this audit baseline is frozen.

## Execution Record

### Implemented

- audited current imports/usages of retained TS recall/reflection helper files;
- confirmed `src/reflection-recall.ts` and `src/reflection-recall-final-selection.ts` are not currently on the active production path;
- confirmed `src/auto-recall-final-selection.ts` remains production code, but only as a prompt-local post-selection seam;
- froze the primary cleanup targets as test/reference relocation plus naming/location debt reduction rather than authority migration.

### Evidence

- `rg -n "recall-engine|auto-recall-final-selection|final-topk-setwise-selection|reflection-recall|reflection-recall-final-selection|adaptive-retrieval" src test index.ts`
- source inspection:
  - `src/recall-engine.ts`
  - `src/adaptive-retrieval.ts`
  - `src/auto-recall-final-selection.ts`
  - `src/final-topk-setwise-selection.ts`
  - `src/reflection-recall.ts`
  - `src/reflection-recall-final-selection.ts`

### Phase 1 checkpoint result

- completed: the residual TS helper set is now classified and future cleanup can target clarity debt instead of re-litigating whether old authority logic still exists.
