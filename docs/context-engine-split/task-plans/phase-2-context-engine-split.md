---
description: Internal orchestration extraction for context-engine-split.
---

# Tasks: context-engine-split

## Input
- `docs/context-engine-split/context-engine-split-implementation-research-notes.md`
- `docs/context-engine-split/context-engine-split-contracts.md`
- `index.ts`
- `src/recall-engine.ts`
- `src/reflection-recall.ts`
- `src/auto-recall-final-selection.ts`
- `src/reflection-store.ts`
- `src/adaptive-retrieval.ts`

## Canonical architecture / Key constraints
- `index.ts` should become a thin wiring layer.
- Backend ownership of storage/retrieval/scopes/tools must remain unchanged.
- New orchestration modules must be ContextEngine-ready but not require a new shipped plugin contract in this branch.
- Do not change public config keys or tool names.

## Format
- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 2: Internal orchestration extraction
Goal: extract prompt-time planning/rendering/state from `index.ts` into dedicated modules while preserving behavior.
Definition of Done: new orchestration/provider modules exist, `index.ts` delegates to them, and tests compile/run for the touched paths.

Tasks:
- [ ] T101 [Backend] Extract generic auto-recall candidate planning into dedicated context/orchestration modules.
  - DoD: a new module owns generic recall candidate loading/filtering/selection composition; `index.ts` no longer contains the bulk of that inline orchestration logic.
- [ ] T102 [Backend] Extract reflection recall and error-signal prompt planning into dedicated modules.
  - DoD: reflection recall candidate planning and error reminder block preparation are delegated out of `index.ts`, with stable hook semantics preserved.
- [ ] T103 [Backend] Extract session-local exposure state into a dedicated service module.
  - DoD: session dedupe/suppression state for recall/reflection/error hints is no longer implemented as ad hoc state ownership in `index.ts`.
- [ ] T104 [P] [QA] Update/add targeted tests for extracted orchestration modules.
  - DoD: touched test files cover the new module boundaries or preserve parity through existing tests, with concrete commands recorded.
- [ ] T105 [Security] Preserve scope filtering and untrusted-data wrapping in the extracted path.
  - DoD: code review/test evidence shows rendering cannot bypass scope filtering or downgrade current untrusted-data framing.

Checkpoint: prompt orchestration is modular and `index.ts` is visibly thinner without changing the external plugin contract.

## Dependencies & Execution Order
- Phase 2 depends on Phase 1.
- T101-T103 may overlap only if file ownership is clearly separated.
- T104 and T105 depend on the extraction landing first.
