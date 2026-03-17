## Context

`openclaw-chronicle-engine` already ships with Rust/backend-owned memory authority. A follow-up source scan showed that `src/` still contains a cluster of `reflection*` helper files from older local-authority stages even though the active runtime now centers on backend transport plus prompt-local orchestration.

## Findings

- Active runtime reflection/recall flow is limited to `index.ts`, `src/context/reflection-prompt-planner.ts`, `src/context/reflection-error-signals.ts`, `src/context/auto-recall-orchestrator.ts`, `src/prompt-local-auto-recall-selection.ts`, `src/recall-engine.ts`, and `src/adaptive-retrieval.ts`.
- The following `src/` files have no active runtime entrypoint and are either dead outright or only retained for narrow test coverage:
  - `src/reflection-aggregation.ts`
  - `src/reflection-selection.ts`
  - `src/reflection-mapped-metadata.ts`
  - `src/reflection-normalize.ts`
  - `src/reflection-ranking.ts`
  - `src/reflection-event-store.ts`
- The following files are not on the runtime entry path and appear to be test-oriented or type-residue only:
  - `src/reflection-metadata.ts`
  - `src/reflection-retry.ts`
  - `src/reflection-item-store.ts`
  - `src/reflection-slices.ts`
- Current non-doc references outside `src/` are limited to tests:
  - `test/memory-reflection.test.mjs` imports `src/reflection-metadata.ts` and `src/reflection-retry.ts`
  - `test/self-improvement.test.mjs` imports `src/reflection-slices.ts`

## Goals / Non-goals

Goals:
- remove dead local-authority reflection residue from `src/`;
- relocate or inline any still-needed test-only helpers so `src/` reflects current runtime ownership;
- keep active prompt-local reflection/recall orchestration intact.

Non-goals:
- no backend API changes;
- no change to shipped recall/reflection runtime behavior;
- no large-scale test refactor beyond the minimum needed to sever deleted `src/` imports.

## Target files / modules

- `src/reflection-aggregation.ts`
- `src/reflection-selection.ts`
- `src/reflection-mapped-metadata.ts`
- `src/reflection-normalize.ts`
- `src/reflection-ranking.ts`
- `src/reflection-event-store.ts`
- `src/reflection-metadata.ts`
- `src/reflection-retry.ts`
- `src/reflection-item-store.ts`
- `src/reflection-slices.ts`
- `test/memory-reflection.test.mjs`
- `test/self-improvement.test.mjs`
- optional new `test/helpers/*` files if a helper must be retained outside runtime `src/`

## Constraints

- Keep this as a small single-contract cleanup scope.
- Do not break the active planner/orchestrator chain rooted at `index.ts`.
- Prefer deletion over relocation when a helper is demonstrably unnecessary.
- If a helper is kept only for tests, move it under `test/helpers/` rather than leaving it in `src/`.

## Verification plan

- `npm test`
- `rg -n "reflection-aggregation|reflection-selection|reflection-mapped-metadata|reflection-normalize|reflection-ranking|reflection-event-store|reflection-metadata|reflection-retry|reflection-item-store|reflection-slices" src test README.md README_CN.md package.json --glob '!docs/**'`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/reflection-src-residual-cleanup-2026-03-17`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/reflection-src-residual-cleanup-2026-03-17 README.md`

## Rollback

- Restore deleted `src/reflection*` files if a supposedly dead helper turns out to back a live runtime path.
- Revert any test-helper relocation if the moved helper still belongs to supported runtime behavior.

## Open questions

- Whether `ReflectionItemKind` should be inlined into `src/context/reflection-prompt-planner.ts` or moved to a smaller neutral types file if `src/reflection-item-store.ts` is removed.

## Implementation Notes

- Deleted dead `src/` residue: `reflection-aggregation`, `reflection-selection`, `reflection-mapped-metadata`, `reflection-normalize`, `reflection-ranking`, and `reflection-event-store`.
- Removed `src/reflection-item-store.ts` and inlined `ReflectionItemKind` into `src/context/reflection-prompt-planner.ts`.
- Moved test-only helpers out of runtime `src/` into `test/helpers/`:
  - `reflection-metadata-reference.ts`
  - `reflection-retry-reference.ts`
  - `reflection-slices-reference.ts`
- Updated `test/memory-reflection.test.mjs` and `test/self-improvement.test.mjs` to import those helper references instead of deleted `src/` modules.
- Resolved the open question by inlining `ReflectionItemKind` into the planner because no other supported runtime path needed a shared `src/` reflection-item type module.

## Evidence

- `npm test` -> passed, `71/71` green after deleting the `src/reflection*` residue cluster.
- `rg -n "reflection-aggregation|reflection-selection|reflection-mapped-metadata|reflection-normalize|reflection-ranking|reflection-event-store|reflection-metadata|reflection-retry|reflection-item-store|reflection-slices" src test README.md README_CN.md package.json --glob '!docs/**'`
  - result: remaining matches exist only in `test/helpers/*-reference.ts` imports from active tests; no active `src/` residue remains for those names.
