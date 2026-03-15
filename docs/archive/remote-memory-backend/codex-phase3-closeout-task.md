# Codex Task: remote-memory-backend Phase 3 closeout

## Context
- Repo: `/root/verify/memory-lancedb-pro-context-engine-split`
- Active branch/worktree already contains Phase 3 integration work.
- Previous supervised run to continue from: `20260313T084019Z-memory-lancedb-pro-context-engine-split-write`
- Current human direction: finish the currently blocking parts first, then push the repo to Phase 4 readiness.

## Current assessment
Phase 3 core integration has landed, but closeout is still blocked by evidence gaps rather than architecture uncertainty.

Confirmed strengths already present:
- backend client seam exists under `src/backend-client/`
- remote tool path exists in `src/backend-tools.ts`
- local orchestration split exists in `src/context/*`
- `/new` and `/reset` reflection path is async enqueue in remote mode
- local shell reflection test suite currently passes

Current blockers to close out:
1. `Shell integration completed` cannot yet be honestly checked because remote-backend-enabled paths are not proven by focused tests.
2. `End-to-end verification completed` cannot yet be checked because the remote adapter / tool / enqueue paths lack direct verification evidence.
3. Docs/checklist need to reflect real closeout evidence once the above is done.

## Goal
Finish the real blocking work for Phase 3 closeout and leave the repo in a state that is ready to enter Phase 4 verification work.

## Required outcomes
### A. Add missing verification for remote-enabled shell paths
Implement focused tests that directly exercise the remote backend integration seams, not only the local orchestration path.
Target areas:
- remote `memory_recall` / `memory_store` / `memory_forget` / `memory_update` tool registration + behavior
- remote `auto-capture` forwarding path
- remote reflection recall path inside `before_prompt_build`
- remote `/new` / `/reset` async reflection enqueue path
- runtime context construction and preservation of:
  - `userId`
  - `agentId`
  - `sessionId`
  - `sessionKey`
- explicit proof that shell does not rebuild local scope authority in remote mode

### B. Tighten any implementation gaps discovered by those tests
If the new tests reveal defects or ambiguities, patch them.
Prefer minimal, contract-preserving fixes.
Do not redesign architecture unless strictly necessary.

### C. Update Phase 3 / Phase 4 planning artifacts
Update the repo-task-driven docs/checklists so they reflect the actual post-fix state.
At minimum review and update:
- `docs/remote-memory-backend/task-plans/4phases-checklist.md`
- `docs/remote-memory-backend/task-plans/phase-3-remote-memory-backend.md`
- `docs/remote-memory-backend/task-plans/phase-4-remote-memory-backend.md`

## Constraints
- Keep `src/context/*` local; do not move memory authority back into shell.
- Keep adapter layer thin and transport-focused.
- Preserve frozen semantics:
  - `sessionKey` = stable logical provenance identity
  - `sessionId` = runtime/diagnostic identity
- No mixed-authority fallback behavior.
- Recall remains fail-open.
- Explicit write/update/delete failures remain surfaced.
- Reflection enqueue remains non-blocking.
- Do not touch `/root/.codex` or global Codex config.

## Deliverables
1. Code and tests that close the current evidence gap for remote shell integration.
2. Updated docs/checklist showing what is now complete vs still Phase 4 work.
3. A concise final status summary with:
   - whether `Backend implementation completed` can now be checked
   - whether `Shell integration completed` can now be checked
   - whether `End-to-end verification completed` can now be checked
   - what remains for Phase 4

## Verification expectation
Run repo verification after changes. Prefer meaningful verification over shallow confirmation.
If possible, reach at least `test` verification with repo commands and report any remaining `check/confirm` gaps honestly.
