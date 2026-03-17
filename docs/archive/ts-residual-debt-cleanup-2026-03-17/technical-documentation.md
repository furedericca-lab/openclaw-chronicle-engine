---
description: Canonical technical architecture for ts-residual-debt-cleanup-2026-03-17.
---

# ts-residual-debt-cleanup-2026-03-17 Technical Documentation

## Canonical architecture

This scope does not change the supported runtime architecture:

1. `backend/src/*` remains the authority layer.
2. `index.ts`, `src/backend-client/*`, and `src/backend-tools.ts` remain the adapter layer.
3. `src/context/*` remains the prompt-time orchestration layer.

The purpose of this scope is narrower:

- reduce misleading TS file placement and naming debt;
- make the remaining local seams obviously prompt-local or test-only;
- prevent future confusion about whether old TS authority code is still present.

## Key constraints and non-goals

- no cleanup may move authority back out of the backend;
- no cleanup may widen ordinary runtime DTOs or alter backend contracts;
- deleting or moving helpers requires test-backed evidence that they are not active production authority paths.

## Cleanup target map

### Category A — production prompt-local seams

These are used in active runtime paths and should be retained unless split/renamed:

- `src/recall-engine.ts`
- `src/adaptive-retrieval.ts`
- `src/prompt-local-auto-recall-selection.ts`
- `src/prompt-local-topk-setwise-selection.ts`

### Category B — test/reference-only helpers

These are now isolated outside top-level `src/`:

- `test/helpers/reflection-recall-reference.ts`
- `test/helpers/reflection-recall-selection-reference.ts`

### Category C — canonical local orchestration modules

These are not debt by themselves and should remain first-class runtime code:

- `src/context/auto-recall-orchestrator.ts`
- `src/context/reflection-prompt-planner.ts`
- `src/context/session-exposure-state.ts`
- `src/context/prompt-block-renderer.ts`

## Operational behavior

Current runtime behavior that cleanup must preserve:

- backend recall returns authoritative rows;
- local planners decide whether and how to inject those rows into prompt blocks;
- prompt-local post-selection may trim already-returned rows for prompt quality but must not redefine backend authority.

## Observability and error handling

This scope should not add new runtime observability surfaces.

The main observability need is auditability of the cleanup itself:

- import/use-site evidence;
- test pass results before and after movement/renaming;
- explicit disposition for each retained helper.

## Security model and hardening notes

- avoid accidental reuse of test/reference helpers in production imports;
- preserve principal-boundary and scope-boundary behavior by keeping cleanup outside backend contracts;
- keep test/reference helpers under `test/helpers/` so no production import path can accidentally pick them up from top-level `src/`.

## Test strategy mapping

Current tests already touching the target surface:

- `test/memory-reflection.test.mjs`
- `test/remote-backend-shell-integration.test.mjs`

Future cleanup should prove:

- runtime production paths still import only prompt-local seams;
- test/reference helpers remain available only where needed;
- no deleted or moved helper breaks README-described behavior or runtime orchestration.
