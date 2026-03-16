# Final Closeout Audit Contract

## Context

The repository has just completed major work across two intersecting tracks:
- `remote-memory-backend`
- `context-engine-split`

The goal of this task is **not** to implement more feature work.
The goal is to perform a broad repo-aware quality audit and identify what still needs cleanup, consolidation, or refactor after these two tracks landed.

This is a closeout-quality review task.

Work in this repo/worktree only:
- repo: `/root/verify/memory-lancedb-pro-context-engine-split`
- branch: current checked-out branch

## Audit goals

Perform a repo-aware audit that answers:
1. What is complete and structurally sound now?
2. What old content / stale code / duplicate pathways / misleading docs still remain?
3. What should be cleaned up before merge or shortly after merge?
4. Which items are true blockers vs important follow-up vs optional polish?

## Focus areas

### A. Architecture consistency
Review whether `remote-memory-backend` and `context-engine-split` currently compose cleanly.
Check for:
- old local-only assumptions that still leak into remote-aware paths
- duplicated orchestration logic still split awkwardly between `index.ts` and `src/context/*`
- leftover naming or API shapes that imply an outdated architecture
- stale comments that no longer match the implemented split

### B. Legacy / dead / duplicate logic
Look for cleanup candidates such as:
- helper code kept only for the old shape but now superseded
- branches, adapters, or compatibility glue that can likely be simplified
- duplicate prompt-block / recall / reflection handling paths
- config compatibility branches that are still necessary vs no longer worth carrying
- comments/docs/test names that still reflect earlier semantics

### C. Docs and naming drift
Review whether docs and naming are now coherent across:
- `README.md`
- `README_CN.md`
- `docs/context-engine-split/*`
- `docs/remote-memory-backend/*`
- code module names and inline comments

Specifically flag:
- obsolete wording
- duplicate docs that should be merged or archived
- docs that still describe superseded behavior
- cases where “context-engine-split” or “remote-memory-backend” language is now misleading for the final architecture

### D. Test quality / verification shape
Assess whether current tests cover the final contract well enough and identify:
- strong coverage areas
- weak or missing closeout coverage
- tests that are now redundant or overfitted to transitional behavior
- places where test names/fixtures still encode old architecture assumptions

### E. Merge-closeout recommendations
Produce a practical closeout list:
- must-clean-before-merge
- should-clean-soon-after-merge
- optional polish

## Constraints

- Default mode is review/audit only. Do not make code changes unless absolutely necessary for a tiny doc-note or evidence aid, and prefer not to patch.
- Focus on high-signal findings. Do not dump a giant low-value nit list.
- Every serious finding must point to concrete files/regions.
- Separate true blockers from cleanup/refactor recommendations.

## Suggested inspection targets

Core code:
- `index.ts`
- `src/context/*`
- `src/backend-client/*`
- `src/backend-tools.ts`
- `src/tools.ts`
- `src/retriever.ts`
- `src/reflection-*`
- `openclaw.plugin.json`

Tests:
- `test/remote-backend-shell-integration.test.mjs`
- `test/memory-reflection.test.mjs`
- `test/config-session-strategy-migration.test.mjs`
- adjacent tests that encode compatibility behavior

Docs:
- `README.md`
- `README_CN.md`
- `docs/context-engine-split/*`
- `docs/remote-memory-backend/*`

## Deliverable format

Return a concise but high-signal audit in this shape:

### 1. Overall assessment
- short paragraph
- verdict: clean | mostly-clean | needs-follow-up | blocked

### 2. Blockers
- only real merge blockers

### 3. Important cleanup / refactor items
For each item:
- file(s)
- what is stale / duplicated / misleading
- why it matters
- recommended cleanup direction

### 4. Optional polish
- lower-priority tidy-ups

### 5. Suggested closeout plan
- immediate pre-merge cleanup
- post-merge follow-up

### 6. Confidence / caveats
- anything that could not be fully confirmed by source inspection alone

## Verification

Source inspection is primary.
You may run targeted repo search / diff / test inspection commands as needed.
If you cite tests, name the exact files/suites involved.
