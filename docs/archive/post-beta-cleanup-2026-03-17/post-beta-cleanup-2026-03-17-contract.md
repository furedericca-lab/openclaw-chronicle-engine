## Context

`openclaw-chronicle-engine` has already completed the backend-authority migration and the `1.0.0-beta.0` plugin/package cutover. The remaining debt is narrow cleanup work:
- active top-level historical docs still read too much like current canonical docs and still mention removed paths/compatibility guarantees;
- one active test file name still advertises the old migration window;
- a few test strings still use migration-era compatibility wording;
- self-improvement default templates still use placeholder serial markers.

## Findings

- `docs/context-engine-split-2026-03-15/` is intentionally top-level, but its README and technical docs still present themselves as active/canonical and still reference `src/reflection-store.ts`.
- `package.json` still runs `test/config-session-strategy-migration.test.mjs` even though the repo baseline is now post-cutover.
- `test/memory-reflection.test.mjs` still contains migration-era wording (`fixed for compatibility`, `compatibility fields`) in active assertions.
- `src/self-improvement-files.ts` still ships `LRN-YYYYMMDD-XXX` / `ERR-YYYYMMDD-XXX` placeholder serial patterns.

## Goals / Non-goals

Goals:
- make active historical docs explicitly non-canonical and update the most misleading stale references;
- rename the session-strategy cutover test file and update all active references;
- remove migration-era wording from active test/README/template surfaces where it no longer reflects the shipped baseline;
- verify the repo remains green after the cleanup.

Non-goals:
- no runtime behavior changes;
- no backend API changes;
- no archive reorganization beyond clarifying current top-level historical snapshot docs.

## Target files / modules

- `docs/README.md`
- `docs/context-engine-split-2026-03-15/README.md`
- `docs/context-engine-split-2026-03-15/context-engine-split-2026-03-15-technical-documentation.md`
- `docs/context-engine-split-2026-03-15/context-engine-split-implementation-research-notes.md`
- `README.md`
- `package.json`
- `test/config-session-strategy-cutover.test.mjs`
- `test/memory-reflection.test.mjs`
- `src/self-improvement-files.ts`

## Constraints

- Keep cleanup bounded to wording, file layout, and documentation clarity.
- Preserve historical records in `docs/archive/` unchanged unless a direct active-reference fix is required.
- Use the existing `repo-task-driven` documentation workflow and keep evidence in this contract.

## Verification plan

- `npm test`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/post-beta-cleanup-2026-03-17`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/post-beta-cleanup-2026-03-17 README.md`
- `rg -n "config-session-strategy-migration\\.test\\.mjs|LRN-YYYYMMDD-XXX|ERR-YYYYMMDD-XXX|fixed for compatibility" README.md README_CN.md docs src test package.json --glob '!docs/archive/**' --glob '!docs/archive/post-beta-cleanup-2026-03-17/**'`

## Rollback

- Revert this cleanup commit as a single unit if any test path or documentation link regression is found.
- No data migration or schema rollback is required.

## Open questions

- None. This cleanup is intentionally bounded and local to wording/layout debt.

## Implementation log

- Reframed `docs/context-engine-split-2026-03-15` as a historical top-level snapshot instead of a canonical active architecture spec.
- Updated the historical snapshot docs to stop pointing readers at removed file layout and old test filenames without rewriting the historical design record into a new scope.
- Renamed the session-strategy test file from `config-session-strategy-migration.test.mjs` to `config-session-strategy-cutover.test.mjs` and aligned the active test script.
- Removed the last active migration-era wording from test names and default self-improvement templates.

## Evidence

- `npm test`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/post-beta-cleanup-2026-03-17`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/post-beta-cleanup-2026-03-17 README.md`
- `npm test` -> passed, 93/93 tests green.
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/post-beta-cleanup-2026-03-17` -> `[OK] placeholder scan clean`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/post-beta-cleanup-2026-03-17 README.md` -> `[OK] post-refactor text scan passed`
- `rg -n "config-session-strategy-migration\\.test\\.mjs|LRN-YYYYMMDD-XXX|ERR-YYYYMMDD-XXX|fixed for compatibility" README.md README_CN.md docs src test package.json --glob '!docs/archive/**' --glob '!docs/archive/post-beta-cleanup-2026-03-17/**'` -> no matches
