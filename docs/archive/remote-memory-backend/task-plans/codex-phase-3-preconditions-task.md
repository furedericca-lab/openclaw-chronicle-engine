Phase 3 precondition gate for remote-memory-backend in this worktree.

Repo: /root/verify/memory-lancedb-pro-context-engine-split
Branch: dev/context-engine-split

Read first:
- docs/remote-memory-backend/phase-2-sign-off-note.md
- docs/remote-memory-backend/remote-memory-backend-contracts.md
- docs/remote-memory-backend-2026-03-17/remote-memory-backend-2026-03-17-technical-documentation.md
- docs/remote-memory-backend/task-plans/phase-3-remote-memory-backend.md
- docs/remote-memory-backend/task-plans/4phases-checklist.md

This task is NOT the main Phase 3 integration yet.
It is the Phase 3 precondition gate and must finish before T201-T205 begin.

Goal:
- obtain a credible verification stamp for the current Phase 2 backend patch in a more stable execution path;
- document any remaining environment blockers explicitly before main Phase 3 integration starts.

Required outcomes:
1. Use a more stable command path to run backend verification.
2. Run `cargo check` in a way that reduces prior linker/resource/outer-run instability.
3. Run targeted backend tests for the highest-risk Phase 2 semantics:
   - auth-context binding
   - LanceDB persistence
   - idempotency lifecycle
   - reflection job ownership
4. Avoid repeating the earlier outer `SIGTERM` failure mode.
5. Update `docs/remote-memory-backend/task-plans/4phases-checklist.md` with:
   - exact commands
   - exact outcomes
   - whether main Phase 3 tasks are cleared to start
   - any environment-blocked note if full verification still cannot be trusted

Constraints:
- Do not start T201-T205 main shell integration in this run unless the precondition gate is clearly satisfied first and documented.
- Do not reopen frozen contracts unless verification reveals a hard contradiction.
- Prefer targeted, stable verification over expensive full rebuild loops if the environment remains fragile.

Preferred report:
- status
- commands used
- verification results
- whether Phase 3 main tasks are cleared
- remaining blockers if not cleared
