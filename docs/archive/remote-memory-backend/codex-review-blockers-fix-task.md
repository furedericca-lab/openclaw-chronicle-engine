# Codex Task: remote-memory-backend blocker remediation after diff review

## Context

A reviewer diff audit against `docs/remote-memory-backend/*` found that the current shell implementation is close to the intended split, but it still violates two important contract points.

This task is to fix the real blockers in code/tests/docs without redesigning the architecture.

Work in this repo/worktree only:
- repo: `/root/verify/memory-lancedb-pro-context-engine-split`
- branch: current checked-out branch

## Reviewer findings to fix

### Blocker 1 — remote mode still eagerly initializes local LanceDB/embedder/retriever and still requires embedding config

Evidence:
- `index.ts` currently performs local storage-path validation and constructs `MemoryStore`, `embedder`, `retriever`, `AccessTracker`, and `migrator` before remote/local mode has meaningfully split.
- `openclaw.plugin.json` still requires `embedding` at the top-level schema.

Why this is a blocker:
- `docs/remote-memory-backend/technical-documentation.md` states:
  - remote backend is the only authority for ACL/scope/provider config/persistence
  - shell does not implement local fallback backend behavior
  - shell startup in remote mode should load only remote transport config plus local integration flags
- If remote mode still depends on local DB/embedding initialization, then remote-only deployment is not actually a thin shell.

Required outcome:
1. In remote mode, do **not** eagerly initialize:
   - local storage path validation
   - `MemoryStore`
   - embedder
   - retriever
   - access tracker
   - migrator
   - local CLI surfaces that require those components
2. Keep local orchestration modules active in remote mode where intended (`src/context/*`), but make them depend only on remote adapter paths.
3. Update config/schema behavior so remote mode does not require local embedding config to be present just to start.
4. Preserve local-mode behavior unchanged.

Preferred repair shape:
- Split startup into two explicit branches:
  - local-authority branch: initializes store/embedder/retriever/access tracker/migrator/local CLI/tools
  - remote-authority branch: initializes remote client + remote tools + local context orchestration only
- Keep shared helper wiring only where it is truly authority-neutral.
- Minimize diff size; do not redesign unrelated plugin surfaces.

### Important 2 — synthetic principal fallback for userId/agentId violates trusted runtime identity contract

Evidence:
- `src/backend-client/runtime-context.ts` falls back to configured/default `userIdFallback` and `agentIdFallback` when runtime identity is missing.
- `openclaw.plugin.json` exposes these fallback values as normal config.
- tests currently treat this behavior as valid.

Why this matters:
- `docs/remote-memory-backend/remote-memory-backend-contracts.md` and `technical-documentation.md` define `X-Auth-User-Id` / `X-Auth-Agent-Id` as trusted runtime principal headers.
- The shell must not silently invent a principal and then send that synthesized identity as if it were authoritative.
- `sessionId` may be generated for diagnostics; `userId` / `agentId` should not be fabricated for data-plane ownership.

Required outcome:
1. `userId` and `agentId` for remote data-plane calls must come from real runtime context.
2. If a remote path lacks principal identity:
   - recall paths should fail open / skip with a warning
   - write/update/delete/list/stats/job-enqueue paths should fail closed with a clear error
3. `sessionId` may still be generated if absent.
4. Update tests to encode the corrected contract.
5. Update docs/schema/UI hints to remove or de-emphasize misleading synthetic-principal behavior.

Preferred repair shape:
- Replace fallback principal generation with explicit missing-identity detection.
- Keep `sessionKey -> agentId` parsing only as a best-effort helper when it reflects real runtime identity and does not fabricate ownership.
- Use precise errors/messages so operators can tell whether a path skipped or failed because runtime identity was unavailable.

## Constraints

- Do not introduce mixed-authority fallback behavior.
- Do not reintroduce shell-side scope authority.
- Keep the remote backend contract aligned with the current docs unless a doc mismatch is unavoidable; if so, update the docs in the same batch.
- Avoid unrelated cleanup.
- Keep existing local-mode tests/behavior intact unless a targeted update is required by the contract fix.

## Expected file targets

Likely code files:
- `index.ts`
- `openclaw.plugin.json`
- `src/backend-client/runtime-context.ts`
- `src/backend-client/client.ts` (only if needed)
- `src/backend-tools.ts`
- `src/context/auto-recall-orchestrator.ts`
- `src/context/reflection-prompt-planner.ts`
- relevant tests under `test/`

Likely docs to update:
- `docs/remote-memory-backend/technical-documentation.md`
- `docs/remote-memory-backend/remote-memory-backend-contracts.md`
- `docs/remote-memory-backend/phase-4-verification-status-report.md` only if the closeout statement must be corrected
- optionally add a concise follow-up status note if that is cleaner than mutating old report language

## Verification requirements

Run meaningful repo verification after changes.

Minimum target:
1. targeted tests for remote shell behavior
2. default repo tests if feasible
3. any schema/config validation that proves remote mode no longer needs local embedding config

Suggested commands:
- `node --test --test-name-pattern='remote backend shell integration|memory reflection' test/remote-backend-shell-integration.test.mjs test/memory-reflection.test.mjs`
- `npm test`

Also verify by inspection/assertion that:
- remote-mode startup no longer constructs local storage/embedder/retriever path
- remote-mode config no longer requires `embedding`
- missing remote principal identity is not silently synthesized

## Deliverable

Return a compact result with:
- status
- files changed
- blocker 1 fix summary
- identity-contract fix summary
- verification commands + results
- remaining gaps or follow-up risks if any
