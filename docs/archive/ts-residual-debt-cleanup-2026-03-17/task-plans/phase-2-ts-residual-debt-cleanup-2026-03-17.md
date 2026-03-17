---
description: Relocate or isolate test/reference-only TS helpers from top-level runtime-oriented source roots.
---

# Tasks: ts-residual-debt-cleanup-2026-03-17

## Input

- `docs/archive/ts-residual-debt-cleanup-2026-03-17/ts-residual-debt-cleanup-2026-03-17-implementation-research-notes.md`
- `docs/archive/ts-residual-debt-cleanup-2026-03-17/ts-residual-debt-cleanup-2026-03-17-technical-documentation.md`
- `docs/archive/ts-residual-debt-cleanup-2026-03-17/ts-residual-debt-cleanup-2026-03-17-contracts.md`
- `src/reflection-recall.ts`
- `src/reflection-recall-final-selection.ts`
- `test/memory-reflection.test.mjs`

## Canonical architecture / Key constraints

- backend authority remains unchanged;
- this phase isolates test/reference-only helpers only;
- production runtime imports must remain clean and explicit.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 2: Reference Helper Isolation

Goal: move test/reference-only helpers out of misleading top-level runtime locations with minimal behavioral risk.

Definition of Done: `src/reflection-recall.ts` and `src/reflection-recall-final-selection.ts` no longer exist in top-level `src/`, their test replacements live under `test/helpers/`, and affected tests are updated.

Tasks:

- [x] T101 [Frontend] Move or isolate `src/reflection-recall.ts` and `src/reflection-recall-final-selection.ts`.
  - DoD: the files are relocated to a clearer test/reference location, and production imports remain absent.
- [x] T102 [P] [QA] Update tests and import paths for the moved or isolated helpers.
  - DoD: `test/memory-reflection.test.mjs` uses the new helper paths and passes.
- [x] T103 [Security] Reconfirm no production import path depends on the relocated helpers.
  - DoD: import scans and tests show the files are not on runtime production paths.

Checkpoint: test/reference helper residue no longer occupies ambiguous top-level runtime space.

## Dependencies & Execution Order

- Phase 2 depends on the frozen audit from Phase 1.
- `T102` depends on the chosen movement/isolation approach from `T101`.

## Execution Record

### Implemented

- moved `src/reflection-recall.ts` to `test/helpers/reflection-recall-reference.ts`;
- moved `src/reflection-recall-final-selection.ts` to `test/helpers/reflection-recall-selection-reference.ts`;
- updated `test/memory-reflection.test.mjs` to import the relocated reference helper;
- confirmed no production runtime import path references either relocated helper.

### Evidence

- `rg -n "reflection-recall-reference|reflection-recall-selection-reference|reflection-recall\\.ts|reflection-recall-final-selection" src test index.ts`
- `node --test --test-name-pattern='.' test/memory-reflection.test.mjs`

### Phase 2 checkpoint result

- completed: test/reference helpers are isolated under `test/helpers/` and no longer read like active runtime authority modules.
