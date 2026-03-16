---
description: Phase 3 execution plan for deleting legacy local-authority modules and rewriting affected tests.
---

# Tasks: remote-authority-reset (Phase 3)

## Input
- `docs/remote-authority-reset/remote-only-local-authority-removal-plan.md`
- `index.ts`
- `src/store.ts`
- `src/retriever.ts`
- `src/embedder.ts`
- `src/tools.ts`
- `src/migrate.ts`
- `src/scopes.ts`
- `src/access-tracker.ts`
- `cli.ts`
- `src/context/*`
- `src/reflection-store.ts`
- `src/reflection-recall.ts`
- `src/auto-recall-final-selection.ts`
- `test/memory-reflection.test.mjs`
- `test/cli-smoke.mjs`
- `test/migrate-legacy-schema.test.mjs`
- `test/retriever-trace.test.mjs`
- `test/vector-search-cosine.test.mjs`
- `test/embedder-error-hints.test.mjs`
- `test/ollama-no-apikey.test.mjs`
- `test/vllm-provider.test.mjs`
- `test/access-tracker.test.mjs`
- `test/benchmark-runner.mjs`

## Canonical architecture / Key constraints
- Remote backend remains sole memory/RAG authority.
- Local code may remain only as adapter + context-engine + self-improvement governance.
- No deleted local module may remain imported from any runtime or test file.
- Context-engine behavior coverage must remain after deleting local store/retriever modules.

## Format
- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 3 Goal
Physically delete local-authority implementation modules and converge tests/import graph to remote-only + context-engine seams.

Definition of Done:
- Legacy local-authority module set is removed from source tree.
- Permanent modules no longer import local store/retriever/embedder types.
- Test suite no longer depends on local-authority runtime classes.

## Tasks
- [x] T041 [Agentic] Delete core local-authority module set.
  - DoD: `src/store.ts`, `src/retriever.ts`, `src/embedder.ts`, `src/tools.ts`, `src/migrate.ts`, `src/scopes.ts`, `src/access-tracker.ts`, and `cli.ts` are removed.

- [x] T042 [Agentic] Remove transitive type coupling from permanent modules.
  - DoD: `src/context/*`, `src/reflection-store.ts`, `src/reflection-recall.ts`, and `src/auto-recall-final-selection.ts` compile without imports from deleted local modules.

- [x] T043 [P] [QA] Rewrite context-engine tests away from local class monkeypatching.
  - DoD: `test/memory-reflection.test.mjs` validates planner behavior via remote-shaped dependency stubs and keeps single-authority guard coverage.

- [x] T044 [P] [QA] Remove obsolete local-authority test files and benchmark harnesses.
  - DoD: local-only suites and runners are deleted/archived with replacement coverage documented.

- [x] T045 [Security] Verify no mixed-authority import/runtime path remains.
  - DoD: repo-wide search confirms zero imports/usages of deleted local modules in runtime/test code.

## Phase 3 Verification
```bash
for f in src/store.ts src/retriever.ts src/embedder.ts src/tools.ts src/migrate.ts src/scopes.ts src/access-tracker.ts cli.ts; do [ ! -e "$f" ] || echo "still present: $f"; done
rg -n "store\.js|retriever\.js|embedder\.js|migrate\.js|tools\.js|scopes\.js|access-tracker\.js|cli\.js" index.ts src test
node --test test/remote-backend-shell-integration.test.mjs test/memory-reflection.test.mjs test/backend-client-retry-idempotency.test.mjs
git diff --check
```

## Dependencies & Execution Order
- Phase 2 must be complete before Phase 3.
- `T041` and `T042` are coupled and should be staged in small commits.
- `T043`/`T044` depend on `T041`/`T042`.
- `T045` runs last.

Checkpoint:
- Codebase no longer contains executable local-authority implementation paths.

## Execution Notes (2026-03-15, cleanup #2)
- Completed in this batch:
  - deleted `src/tools.ts`, `src/migrate.ts`, `cli.ts`.
  - deleted `test/cli-smoke.mjs`, `test/migrate-legacy-schema.test.mjs`.
  - rewrote `test/memory-reflection.test.mjs` to remove local class monkeypatch suites.
- Deferred in this phase:
  - remaining local modules (`src/store.ts`, `src/retriever.ts`, `src/embedder.ts`, `src/scopes.ts`, `src/access-tracker.ts`) await type-coupling extraction.
  - remaining local-coupled test suites remain pending until module deletions complete.

## Execution Notes (2026-03-16, cleanup #3)
- Completed in this batch:
  - deleted remaining local-authority modules: `src/store.ts`, `src/retriever.ts`, `src/embedder.ts`, `src/scopes.ts`, `src/access-tracker.ts`.
  - deleted coupled local benchmark/runtime surfaces: `src/benchmark.ts`, `test/benchmark-runner.mjs`.
  - deleted local-only tests coupled to removed runtime modules:
    - `test/access-tracker.test.mjs`
    - `test/retriever-trace.test.mjs`
    - `test/vector-search-cosine.test.mjs`
    - `test/embedder-error-hints.test.mjs`
    - `test/ollama-no-apikey.test.mjs`
    - `test/vllm-provider.test.mjs`
  - removed transitive type coupling by introducing `src/memory-record-types.ts` and updating:
    - `src/context/auto-recall-orchestrator.ts`
    - `src/context/reflection-prompt-planner.ts`
    - `src/reflection-store.ts`
    - `src/reflection-recall.ts`
    - `src/auto-recall-final-selection.ts`
  - rewrote planner tests in `test/memory-reflection.test.mjs` to remote-only dependency stubs.
  - updated `package.json` test script to remove deleted local suites.
- Verification evidence (executed):
  - `for f in src/store.ts src/retriever.ts src/embedder.ts src/scopes.ts src/access-tracker.ts; do [ ! -e "$f" ] || echo "still present: $f"; done` -> no output.
  - `node --test test/remote-backend-shell-integration.test.mjs test/memory-reflection.test.mjs test/backend-client-retry-idempotency.test.mjs` -> pass (`78/78`).
  - `npm test` -> pass (`92/92`).
  - `git diff --check` -> clean.
  - legacy-pattern scan command from this file matches only substring false positives (`reflection-store.js`, `backend-tools.js`, `self-improvement-tools.js`) and no deleted-module import edges.
  - strict import scan:
    - `rg -n 'from "\\./(store|retriever|embedder|migrate|tools|scopes|access-tracker)\\.js"|from "\\.\\./(store|retriever|embedder|migrate|tools|scopes|access-tracker)\\.js"|from "\\./cli\\.js"|from "\\.\\./cli\\.js"' index.ts src test`
    - result: no matches.
