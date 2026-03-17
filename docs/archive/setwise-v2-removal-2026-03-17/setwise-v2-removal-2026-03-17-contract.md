## Context

- The plugin still exposes `setwise-v2` as an `autoRecallSelectionMode` option even though the canonical retrieval/ranking/diversity authority now lives in the Rust backend.
- Current `setwise-v2` behavior is limited to prompt-local post-selection over backend-returned rows, but it still keeps extra runtime modules, config schema surface, tests, and README claims alive.
- The requested scope is to remove `setwise-v2` entirely from the active runtime surface.

## Findings

- Runtime selection branching exists in `index.ts` and `src/context/auto-recall-orchestrator.ts`.
- Prompt-local selector implementation lives in `src/prompt-local-auto-recall-selection.ts` and `src/prompt-local-topk-setwise-selection.ts`.
- Active tests still assert explicit support for `setwise-v2` in `test/config-session-strategy-cutover.test.mjs` and `test/memory-reflection.test.mjs`.
- Public config/help/docs still advertise `setwise-v2` in `openclaw.plugin.json`, `README.md`, and `README_CN.md`.

## Goals / Non-goals

- Goals:
- Remove `setwise-v2` from the active plugin config schema and runtime selection modes.
- Collapse generic auto-recall final selection to backend-owned `mmr` semantics plus direct plugin-side truncation only.
- Delete prompt-local `setwise-v2` implementation modules and update tests/docs accordingly.

- Non-goals:
- No backend retrieval/ranking changes.
- No reflection recall behavior changes.
- No archive-wide historical doc rewrites outside this scope doc and active top-level docs.

## Target files / modules

- `index.ts`
- `src/context/auto-recall-orchestrator.ts`
- `src/prompt-local-auto-recall-selection.ts`
- `src/prompt-local-topk-setwise-selection.ts`
- `openclaw.plugin.json`
- `README.md`
- `README_CN.md`
- `test/config-session-strategy-cutover.test.mjs`
- `test/memory-reflection.test.mjs`

## Constraints

- Preserve the current default mode as `mmr`.
- The repo is still pre-release, so removed `setwise-v2` configs may hard-fail instead of being mapped forward.
- Keep the change bounded to plugin/runtime/test/docs surfaces; do not reopen backend contracts.

## Verification plan

- `npm test`
- `rg -n "setwise-v2|prompt-local-auto-recall-selection|prompt-local-topk-setwise-selection" src test index.ts openclaw.plugin.json README.md README_CN.md`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/setwise-v2-removal-2026-03-17`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/setwise-v2-removal-2026-03-17 README.md`

## Rollback

- Restore the deleted prompt-local selector modules.
- Reintroduce the config schema enum entry and parser branch for `setwise-v2`.
- Restore the removed tests and README statements if runtime behavior must be reinstated.

## Open questions

- Whether `CHANGELOG.md` should explicitly record the pre-release removal of `setwise-v2`.

## Implementation status

- Completed: removed `setwise-v2` from active runtime types, parser semantics, schema help text, and top-level READMEs.
- Completed: deleted `src/prompt-local-auto-recall-selection.ts` and `src/prompt-local-topk-setwise-selection.ts`.
- Completed: simplified generic auto-recall to backend-owned `mmr` ranking plus direct local truncation only.
- Completed: replaced the old acceptance test with explicit rejection coverage for `autoRecallSelectionMode=setwise-v2`.

## Evidence

- `npm test`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/setwise-v2-removal-2026-03-17`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/setwise-v2-removal-2026-03-17 README.md`
