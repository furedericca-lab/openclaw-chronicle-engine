# Codex Task: remote-memory-backend Phase 4 verification

## Context
- Repo: `/root/verify/memory-lancedb-pro-context-engine-split`
- Previous supervised run to continue from: `20260313T132711Z-memory-lancedb-pro-context-engine-split-write`
- Phase 3 shell closeout evidence is now landed.
- Reviewer judgment on the latest Codex batch: acceptable quality, no new Phase 3 blocker found.

## What is already true
- Remote shell integration evidence now exists via `test/remote-backend-shell-integration.test.mjs`.
- `npm test` currently passes.
- Phase 3 checklist can credibly mark `Shell integration completed` as done.

## Remaining gaps before stronger sign-off
These are Phase 4 concerns, not reasons to reopen Phase 3:
1. Failure-semantics verification is still incomplete.
2. Admin/control-plane isolation still needs explicit evidence.
3. Contract-hardening edge cases still need direct test coverage and checklist evidence.
4. Release-facing migration / rollback / parity-boundary docs still need Phase 4-level finalization.

## Goal
Continue into Phase 4, clearing the remaining verification/documentation blockers without redesigning the architecture.

## Required outcomes
### A. Execute Phase 4 verification work
Prioritize these tasks from the phase plan:
- `T301` failure-semantics verification
- `T304` contract-hardening edge coverage
- `T303` admin/control-plane isolation verification
- `T302` migration / rollback notes
- `T305` MVP parity boundary / deferred-surface release docs

### B. Add focused tests/evidence where missing
At minimum, cover as directly as practical:
- fail-open recall behavior when backend generic/reflect recall fails
- surfaced write/update/delete failures in remote mode
- non-blocking reflection enqueue semantics, including failure observability expectations
- caller-scoped reflection job visibility semantics
- actor-envelope stats behavior
- list ordering and `nextOffset=null` terminal-page semantics
- rejection of forbidden scope payloads / non-frozen category payloads where those checks belong
- admin-token bypass isolation to explicit management/control-plane routes only

### C. Update docs/checklists/status artifacts
Update at minimum:
- `docs/remote-memory-backend/task-plans/4phases-checklist.md`
- `docs/remote-memory-backend/task-plans/phase-4-remote-memory-backend.md`
- `docs/remote-memory-backend-2026-03-17/remote-memory-backend-2026-03-17-technical-documentation.md`
- create or update a concise Phase 4 status report under `docs/remote-memory-backend/`

## Constraints
- Keep `src/context/*` local.
- Keep adapter layer thin and transport-focused.
- No mixed-authority fallback behavior.
- Recall must remain fail-open.
- Explicit write/update/delete failures must remain surfaced.
- Reflection enqueue must remain non-blocking.
- Do not touch `/root/.codex` or global Codex config.
- Prefer minimal, contract-preserving changes.

## Deliverables
1. Code/tests/docs that materially advance or complete Phase 4.
2. Updated checklist showing exactly which Phase 4 tasks are now complete.
3. Final status summary stating:
   - whether `Backend implementation completed` can now be checked
   - whether `End-to-end verification completed` can now be checked
   - what remains, if anything

## Verification expectation
Run meaningful verification, not just shallow smoke checks.
Reach at least `test` layer with repo commands, and include any backend-side verification commands needed for the claims you make.
Be explicit about any residual gap instead of silently assuming completion.
