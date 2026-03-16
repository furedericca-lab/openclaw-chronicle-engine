# Final Closeout Implementation Contract

## Context

This repository has already completed the main `remote-memory-backend` and `context-engine-split` implementation work, plus the blocker-remediation and contract-closeout passes.

This task is the **final closeout implementation batch**.
It should focus on cleanup, semantic alignment, documentation hygiene, and verification hardening.

Work in this repo/worktree only:
- repo: `/root/verify/memory-lancedb-pro-context-engine-split`
- branch: current checked-out branch

## Goals

Complete the following in one coherent batch:

1. Add remote retry / idempotency tests.
2. Unify local / remote reflection reset contract.
3. Normalize tool `scope` semantics across local vs remote paths.
4. Archive phase / status / task docs and make canonical docs obvious.
5. Apply backend-neutral naming cleanup where appropriate.
6. Tidy test naming / layering where useful and low-risk.
7. Apply small README / README_CN polish consistent with the final state.

## Constraints

- Do not introduce new product scope.
- Preserve the already-verified principal identity contract.
- Preserve the already-verified local-vs-remote authority split.
- Keep changes targeted and reviewable; avoid broad unrelated refactors.
- If a cleanup is too risky for this batch, document it rather than forcing it.

## Required work

### A. Remote retry / idempotency tests
Strengthen remote backend client verification.

Add targeted tests for:
- retryable backend failures (for example 429 / 503) where retries should occur
- non-retryable failures where retries should not occur
- write/enqueue idempotency-key expectations on repeated requests
- any critical transport retry behavior that is currently only implied by code

Target areas:
- `src/backend-client/client.ts`
- `test/remote-backend-shell-integration.test.mjs`
- add a new focused backend-client test file if that is cleaner

### B. Unify local / remote reflection reset contract
Current local and remote `command:new` / `command:reset` reflection behavior should follow one shared contract shape.

Required outcome:
- define the intended behavior clearly in code and docs
- make local and remote paths match that contract as closely as practical in this batch
- avoid config-surface drift where the same `memoryReflection` settings imply materially different behavior by backend mode unless explicitly documented as deferred

Target areas:
- `index.ts`
- `src/context/reflection-prompt-planner.ts`
- related docs/tests

### C. Normalize tool `scope` semantics
Current local vs remote tool semantics should no longer silently drift.

Required outcome:
- either remove deprecated `scope` from the remote tool contract surface and align docs/tests
- or normalize behavior in a clean adapter way so user-visible semantics are obvious and stable

Be pragmatic: choose the least risky path that reduces semantic ambiguity.

Target areas:
- `src/backend-tools.ts`
- `src/tools.ts`
- tests and README/docs that mention scope behavior

### D. Archive phase / status / task docs, highlight canonical docs
Current `docs/context-engine-split/*` and `docs/remote-memory-backend/*` mix canonical docs with historical execution artifacts.

Required outcome:
- preserve history, but move clearly historical docs into an archive/history location
- keep canonical references prominent and easy to discover
- update links if needed

Suggested result shape:
- canonical docs remain in active scope root
- historical task/phase/status/codex-run docs move under something like `docs/archive/...` or `<scope>/history/...`

Target areas:
- `docs/context-engine-split/*`
- `docs/remote-memory-backend/*`
- README references if needed

### E. Backend-neutral naming cleanup
Apply low-risk naming cleanup where user-facing wording still implies purely local LanceDB semantics despite remote-capable architecture.

This applies to:
- comments
- log lines
- user-facing docs/schema/help text

Do not rename core package identity (`memory-lancedb-pro`) in this batch.
Do focus on wording that should be backend-neutral when talking about architecture or mode behavior.

Target areas:
- `openclaw.plugin.json`
- `README.md`
- `README_CN.md`
- selected comments/log strings in code

### F. Test naming / layering cleanup
Perform low-risk cleanup where tests still encode transitional architecture naming.

Examples:
- split overly broad remote backend test file if helpful
- improve suite naming if current names are misleading
- remove obvious duplicate compatibility assertions when safe

Do not destabilize good coverage just for aesthetics.

### G. README small polish
Do a final small polish pass on:
- `README.md`
- `README_CN.md`

Keep current short structure, but improve any wording made awkward by the closeout cleanup above.

## Verification requirements

Run meaningful verification after changes.

Minimum expected verification:
- targeted tests covering the new retry/idempotency work and any reflection/tool contract updates
- `npm test`
- doc/link sanity if docs are moved/archived

Suggested commands:
- `node --test --test-name-pattern='remote|backend|reflection|sessionStrategy' test/remote-backend-shell-integration.test.mjs test/memory-reflection.test.mjs test/config-session-strategy-migration.test.mjs`
- `npm test`
- `git diff --check`

## Deliverable

Return:
- status
- files changed
- what was done for each of the 7 goals
- verification commands + results
- any consciously deferred item with reason
