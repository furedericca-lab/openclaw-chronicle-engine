## Context

`openclaw-chronicle-engine` now ships a Rust backend as the memory authority. The remaining test debt is narrow and local: several test/reference helper files still model old local-authority behavior even though the supported runtime no longer imports those paths.

## Findings

- `test/query-expander.test.mjs` only exercises `test/helpers/query-expander-reference.ts`, which is not imported by the supported runtime.
- `test/memory-reflection.test.mjs` still imports `test/helpers/reflection-store-reference.ts` and `test/helpers/reflection-recall-reference.ts` for historical local reflection behavior checks.
- `test/helpers/reflection-recall-reference.ts` depends on `test/helpers/reflection-recall-selection-reference.ts`, so the whole chain is test-only residue once the historical local-authority assertions are removed.
- `README.md`, `README_CN.md`, and `package.json` still describe or run those reference-only files.

## Goals / Non-goals

Goals:
- remove test/reference files that only preserve obsolete local-authority memory behavior;
- delete or rewrite tests that depend on those files when they no longer validate the shipped runtime;
- update README/docs/test script text so the repo no longer advertises removed reference helpers.

Non-goals:
- no backend API changes;
- no removal of current prompt-local orchestration tests that still cover shipped behavior;
- no change to active backend/shell/config/self-improvement test coverage.

## Target files / modules

- `test/query-expander.test.mjs`
- `test/helpers/query-expander-reference.ts`
- `test/memory-reflection.test.mjs`
- `test/helpers/reflection-store-reference.ts`
- `test/helpers/reflection-recall-reference.ts`
- `test/helpers/reflection-recall-selection-reference.ts`
- `package.json`
- `README.md`
- `README_CN.md`

## Constraints

- Preserve tests that still validate current prompt-local orchestration seams such as planners, prompt block rendering, session-state hygiene, and active config parsing.
- Treat Rust/backend authority as the source of truth; do not keep local-authority regression tests just because they are historically interesting.
- Keep the scope small and auditable.

## Verification plan

- `npm test`
- `rg -n "query-expander-reference|reflection-store-reference|reflection-recall-reference|reflection-recall-selection-reference|query-expander\\.test\\.mjs" README.md README_CN.md package.json test --glob '!docs/archive/**'`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/test-reference-cleanup-2026-03-17`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/test-reference-cleanup-2026-03-17 README.md`

## Rollback

- Revert the cleanup commit if test coverage drops too far or if active runtime behavior turns out to depend on a removed assertion set.
- No schema or data rollback is required.

## Open questions

- Whether any of the historical ranking assertions should be re-expressed against current prompt-local modules instead of being deleted outright.

## Implementation log

- Removed `test/query-expander.test.mjs` and its only backing file `test/helpers/query-expander-reference.ts`.
- Removed `test/helpers/reflection-store-reference.ts`, `test/helpers/reflection-recall-reference.ts`, and `test/helpers/reflection-recall-selection-reference.ts`.
- Trimmed `test/memory-reflection.test.mjs` down to current prompt-local/runtime-relevant coverage by deleting the historical local-authority reflection persistence/ranking sections.
- Updated `package.json`, `README.md`, and `README_CN.md` so the active repo no longer advertises the deleted reference-only test files.

## Evidence

- `npm test` -> passed, 68/68 tests green after removing the reference-only suite.
- `rg -n "query-expander-reference|reflection-store-reference|reflection-recall-reference|reflection-recall-selection-reference|query-expander\\.test\\.mjs" README.md README_CN.md package.json test --glob '!docs/archive/**'` -> no matches
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/test-reference-cleanup-2026-03-17` -> `[OK] placeholder scan clean`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/test-reference-cleanup-2026-03-17 README.md` -> `[OK] post-refactor text scan passed`
