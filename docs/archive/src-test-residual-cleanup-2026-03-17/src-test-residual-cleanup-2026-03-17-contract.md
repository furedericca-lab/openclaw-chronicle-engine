## Context

`openclaw-chronicle-engine` has already completed the remote-backend authority cutover, but a narrow review of `src/` and `test/` still shows a few old-architecture residues:
- the adapter-facing `memory_recall` tool still performs local category filtering after backend recall;
- planner dependency types lag behind the newly shipped backend-owned recall filter fields;
- several test variables and helper strings still use stale `LanceDB` / `embedded` naming from pre-cutover architecture.
- test helper surface is split across three separate `reflection-*` reference files even though they now serve one consolidated test-only reflection support role;
- self-improvement registration is split across `src/self-improvement-tools.ts` and a thin `src/self-improvement-registration.ts` wrapper without any real ownership boundary.

## Findings

- `src/backend-tools.ts` accepts a `category` parameter for `memory_recall` but filters returned rows locally instead of forwarding the filter to backend recall.
- `src/context/auto-recall-orchestrator.ts` and `src/context/reflection-prompt-planner.ts` call dependency functions with backend filter fields that are not declared in their dependency interfaces.
- `test/memory-reflection.test.mjs` and `test/remote-backend-shell-integration.test.mjs` still bind the plugin under the old local-architecture-flavored variable name `memoryLanceDBProPlugin`.
- `test/helpers/reflection-retry-reference.ts` and `test/helpers/reflection-slices-reference.ts` still describe old `embedded` execution wording that no longer matches current runtime architecture.
- `test/helpers/reflection-metadata-reference.ts`, `test/helpers/reflection-retry-reference.ts`, and `test/helpers/reflection-slices-reference.ts` can be collapsed into one `test/helpers/reflection-reference.ts` helper without reducing test readability.
- `src/self-improvement-registration.ts` only re-exports/coordinates tool registration and can be folded into `src/self-improvement-tools.ts`.

## Goals / Non-goals

Goals:
- remove remaining plugin-side category filtering from the runtime recall tool path;
- align planner dependency types with the actual backend-owned recall filter fields in use;
- rename stale test-only identifiers and wording so test coverage reflects current architecture cleanly.
- merge the three test-only reflection reference helpers into one consolidated helper file;
- merge the two self-improvement registration/tool files into one consolidated source file.

Non-goals:
- no backend contract redesign beyond the already shipped additive filter fields;
- no archive sweep across historical docs or archived test fixtures;
- no behavior change to the retained prompt-local `setwise-v2` post-selection seam.

## Target files / modules

- `src/backend-tools.ts`
- `src/context/auto-recall-orchestrator.ts`
- `src/context/reflection-prompt-planner.ts`
- `src/self-improvement-tools.ts`
- `src/self-improvement-registration.ts`
- `test/memory-reflection.test.mjs`
- `test/remote-backend-shell-integration.test.mjs`
- `test/self-improvement.test.mjs`
- `test/helpers/reflection-metadata-reference.ts`
- `test/helpers/reflection-retry-reference.ts`
- `test/helpers/reflection-slices-reference.ts`
- `test/helpers/reflection-reference.ts`

## Constraints

- Backend-visible recall filtering must remain backend-owned.
- Test-only helper wording may reference historical behavior only when it is intentionally validating a removed path; otherwise use current architecture terms.
- Keep the scope small and auditable; prefer renames and direct seam cleanup over broader refactors.
- Consolidation work should preserve existing exported names where practical so test/runtime call sites remain stable.

## Verification plan

- `npm test`
- `rg -n "memoryLanceDBProPlugin|embedded reflection generation|runner: \\\"embedded\\\"|rows.filter\\(\\(row\\) => row.category === category\\)|reflection-metadata-reference|reflection-retry-reference|reflection-slices-reference|self-improvement-registration" src test index.ts`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/src-test-residual-cleanup-2026-03-17`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/src-test-residual-cleanup-2026-03-17 README.md`

## Rollback

- Restore local adapter-side category filtering only if the backend-facing recall tool contract cannot represent the user-facing `category` parameter cleanly.
- Revert test/helper renames if they break searchability for intentionally historical failure semantics.

## Open questions

- Whether `memory_recall` should expose multi-category filtering later, now that backend `categories` is canonical.

## Closeout

Status: completed and archived on 2026-03-17.

Outcome:
- removed plugin-side `memory_recall` category post-filtering and forwarded category filtering to backend recall;
- aligned planner dependency types with the backend-owned generic/reflection filter fields already in active use;
- removed stale `LanceDB` / `embedded` naming from active `src/` and `test/` coverage;
- merged `test/helpers/reflection-metadata-reference.ts`, `test/helpers/reflection-retry-reference.ts`, and `test/helpers/reflection-slices-reference.ts` into `test/helpers/reflection-reference.ts`;
- merged `src/self-improvement-registration.ts` into `src/self-improvement-tools.ts` and kept `registerSelfImprovementTools` as the stable exported entry point.

Verification evidence:
- `npm test`
- `rg -n "memoryLanceDBProPlugin|embedded reflection generation|runner: \\\"embedded\\\"|rows.filter\\(\\(row\\) => row.category === category\\)|reflection-metadata-reference|reflection-retry-reference|reflection-slices-reference|self-improvement-registration" src test index.ts`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/src-test-residual-cleanup-2026-03-17`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/src-test-residual-cleanup-2026-03-17 README.md`
