# Codex Task: remote-memory-backend contract closeout after blocker remediation

## Context

The previous blocker-remediation batch passed tests and removed the main blockers, but reviewer follow-up still found two remaining risks that should be closed before considering the shell/backend contract fully settled.

Work in this repo/worktree only:
- repo: `/root/verify/memory-lancedb-pro-context-engine-split`
- branch: current checked-out branch

## Remaining reviewer risks to close

### Risk 1 — `agentId` is still derivable from `sessionKey`

Current code path:
- `src/backend-client/runtime-context.ts`
- `parseAgentIdFromSessionKey(rawSessionKey)` still contributes to remote data-plane principal resolution.

Why this remains risky:
- The intended remote contract says trusted principal identity comes from runtime/gateway identity fields and trusted headers.
- `sessionKey` may be useful for provenance and correlation, but it should not remain a shadow principal source unless docs explicitly define it as such.
- Leaving this half-open weakens the clarity of the data-plane authority model.

Required outcome:
1. Choose and implement the stricter contract-closeout path:
   - preferred: `agentId` for remote data-plane calls must come from explicit runtime identity fields, not from `sessionKey` parsing.
2. Keep `sessionKey` as provenance/correlation only.
3. Update tests so they no longer encode `sessionKey -> agentId` as accepted principal recovery behavior.
4. Update docs/contracts anywhere they still overstate or understate the final behavior.

Important constraint:
- Do not break valid runtime paths that already provide explicit `agentId`.
- If a path truly lacks explicit agent identity, it should behave consistently with the principal policy:
  - recall-style read paths: fail-open / skip with warning
  - write/update/delete/list/stats/job-enqueue paths: fail-closed

### Risk 2 — local-mode `embedding` requirement moved from schema into runtime-only failure

Current state:
- `openclaw.plugin.json` no longer requires `embedding` at the top-level, which fixed remote mode startup.
- But local mode now relies on runtime code to throw if `embedding` is absent.

Why this remains risky:
- It weakens configuration validation and may let an invalid local-mode config pass schema/UI validation only to fail later at plugin register/startup.
- The remote-mode fix should not silently degrade local-mode config correctness.

Required outcome:
1. Tighten validation so the final behavior is explicit and user-facing:
   - remote mode: `embedding` not required
   - local mode: `embedding` required
2. If the plugin schema system cannot express this condition directly, implement the best available closeout shape:
   - explicit parse/validation-time mode-aware check with a clear error message, not a deep runtime failure
   - docs/UI hints must make the rule obvious
3. Add or update tests proving:
   - remote mode accepts missing `embedding`
   - local mode rejects missing `embedding` early and clearly

## Contract closeout review pass

After fixing the two risks above, perform one more contract-alignment pass against:
- `docs/remote-memory-backend-2026-03-17/remote-memory-backend-2026-03-17-technical-documentation.md`
- `docs/remote-memory-backend/remote-memory-backend-contracts.md`

Closeout goal:
- code, tests, and docs all agree on the final shell authority model for:
  - principal identity
  - `sessionKey` semantics
  - remote-vs-local initialization boundary
  - local-mode configuration requirements

If the current docs still imply behavior that is no longer true, update them in the same batch.
If the code is already correct and docs are the lagging side, prefer doc correction rather than code churn.

## Constraints

- Do not reintroduce synthetic principal fallback.
- Do not reintroduce local fallback backend behavior in remote mode.
- Do not add unrelated refactors.
- Preserve the already-passing blocker-remediation behavior unless required by this closeout.
- Keep the final contract simple and explicit.

## Likely files

Code:
- `src/backend-client/runtime-context.ts`
- `index.ts`
- `src/backend-tools.ts`
- `openclaw.plugin.json`
- relevant parse/config helpers if needed
- `test/remote-backend-shell-integration.test.mjs`

Docs:
- `docs/remote-memory-backend-2026-03-17/remote-memory-backend-2026-03-17-technical-documentation.md`
- `docs/remote-memory-backend/remote-memory-backend-contracts.md`
- optionally a concise follow-up closeout note/status report if useful

## Verification requirements

Run meaningful verification after changes.

Minimum:
- targeted remote shell tests
- any local/parse validation tests needed for the embedding requirement closeout
- full `npm test` if feasible

Suggested commands:
- `node --test --test-name-pattern='remote backend shell integration|sessionStrategy legacy compatibility mapping' test/remote-backend-shell-integration.test.mjs test/memory-reflection.test.mjs test/config-session-strategy-migration.test.mjs`
- `npm test`

## Deliverable

Return a compact result with:
- status
- files changed
- final decision on `sessionKey` vs `agentId` authority
- final decision on local-mode embedding validation
- docs/contracts updated
- verification commands + results
- any remaining contract caveat if one still cannot be removed cleanly
