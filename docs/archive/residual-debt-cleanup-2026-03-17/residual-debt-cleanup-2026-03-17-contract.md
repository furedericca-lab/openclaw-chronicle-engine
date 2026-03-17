## Context

`openclaw-chronicle-engine` already completed the Rust-backend authority migration and the `1.0.0-beta.0` plugin/package cutover. A final repo scan still found a narrow set of residual cleanup items:
- a top-level historical docs snapshot still described itself as canonical/active;
- `docs/documentation-refresh` remained as an empty top-level directory;
- `test/noise-filter-chinese.mjs` lived under `test/` but was not part of `npm test`;
- one schema help string still used migration-era compatibility wording.

## Findings

- `docs/README.md` classifies `docs/remote-memory-backend-2026-03-17/` as a read-only historical snapshot, but that folder README still says `Canonical documents (active)`.
- `docs/documentation-refresh/` exists as an empty directory and no longer carries active or archived content.
- `test/noise-filter-chinese.mjs` is a useful regression check for `src/noise-filter.ts`, but it still uses a manual script format and is absent from `package.json` test wiring.
- `openclaw.plugin.json` still says `fixed keeps compatibility inheritance` for `memoryReflection.recall.mode`, which no longer matches the post-cutover wording baseline.

## Goals / Non-goals

Goals:
- align the top-level snapshot docs with the actual historical-only policy;
- remove the empty top-level docs residue;
- convert the Chinese noise-filter script into normal automated coverage and wire it into `npm test`;
- replace the remaining stale compatibility wording in active schema text.

Non-goals:
- no runtime behavior change to memory authority, retrieval, or reflection execution;
- no new phased scope or architecture work;
- no cleanup inside archived scopes beyond keeping top-level pointers accurate.

## Target files / modules

- `docs/README.md`
- `docs/remote-memory-backend-2026-03-17/README.md`
- `docs/archive/residual-debt-cleanup-2026-03-17/residual-debt-cleanup-2026-03-17-contract.md`
- `docs/documentation-refresh/` (delete empty directory)
- `test/noise-filter-chinese.mjs`
- `package.json`
- `openclaw.plugin.json`

## Constraints

- Keep this as a single-contract cleanup scope.
- Use `apply_patch` for file edits.
- Remove directories with the repo-task-driven `safe_delete_tree.sh` helper rather than `rm -rf`.
- Keep the resulting test readable and deterministic under `node --test`.

## Verification plan

- `npm test`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/residual-debt-cleanup-2026-03-17`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/residual-debt-cleanup-2026-03-17 README.md`
- `test ! -d docs/documentation-refresh`
- `rg -n "Canonical documents \\(active\\)|fixed keeps compatibility inheritance|Usage: node test/noise-filter-chinese.mjs" docs test openclaw.plugin.json package.json`

## Rollback

- Restore the previous README wording if the historical snapshot labeling proves too aggressive.
- Revert the `noise-filter-chinese` test conversion if `node --test` integration proves unstable.
- Recreate `docs/documentation-refresh/` only if a still-active scope unexpectedly depended on that exact top-level path.

## Open questions

- None at scope creation time.

## Implementation Notes

- Scope opened as a bounded cleanup after a repo-wide residual-debt scan on 2026-03-17.
- Updated `docs/remote-memory-backend-2026-03-17/README.md` to match the top-level historical-snapshot policy instead of presenting the folder as canonical active docs.
- Deleted the empty top-level `docs/documentation-refresh/` directory with `safe_delete_tree.sh`.
- Converted `test/noise-filter-chinese.mjs` from a manual script into a normal `node:test` suite and added it to `package.json` `npm test`.
- Reworded `openclaw.plugin.json` `memoryReflection.recall.mode` help text to remove the stale migration-era compatibility phrasing.

## Evidence

- `npm test` -> passed, `71/71` green after wiring in `test/noise-filter-chinese.mjs`.
- `test ! -d docs/documentation-refresh` -> passed.
- `rg -n "Canonical documents \\(active\\)|fixed keeps compatibility inheritance|Usage: node test/noise-filter-chinese.mjs" docs/remote-memory-backend-2026-03-17 test/noise-filter-chinese.mjs openclaw.plugin.json package.json` -> no matches.
