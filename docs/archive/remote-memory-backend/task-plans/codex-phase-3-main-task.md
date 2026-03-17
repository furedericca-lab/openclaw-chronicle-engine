Phase 3 main integration for remote-memory-backend in this worktree.

Repo: /root/verify/memory-lancedb-pro-context-engine-split
Branch: dev/context-engine-split

Read first:
- docs/remote-memory-backend/phase-2-sign-off-note.md
- docs/remote-memory-backend/remote-memory-backend-contracts.md
- docs/remote-memory-backend-2026-03-17/remote-memory-backend-2026-03-17-technical-documentation.md
- docs/remote-memory-backend/task-plans/phase-3-remote-memory-backend.md
- docs/remote-memory-backend/task-plans/4phases-checklist.md
- docs/remote-memory-backend/task-plans/phase-3-implementation-handoff.md

The Phase 3 precondition gate is already complete.
This run is the Phase 3 main task.

Execution order:
1. T201 first
2. Then progress T202-T205 in parallel where practical

Primary goal:
- replace local backend authority with a thin HTTP-backed shell adapter while preserving local `src/context/*` orchestration and session-local behavior.

Required constraints:
- `src/context/*` must remain local.
- Do not regain local ACL/scope authority in the shell.
- Do not send requested scopes, scope overrides, or provider config.
- Preserve frozen runtime route semantics.
- Preserve `sessionKey` as stable logical provenance and `sessionId` as ephemeral diagnostics.
- `/new` and `/reset` must remain non-blocking and enqueue reflection jobs asynchronously.
- Do not expand into deferred CLI/operator surfaces.

T201 minimum bar:
- introduce a local backend adapter/client seam that centralizes:
  - base URL/config loading
  - bearer token handling
  - trusted runtime identity header forwarding
  - actor-envelope request shaping
  - retry/error translation
- this seam must exist before broad rewiring starts.

T202-T205 focus after T201 lands:
- T202: generic recall + reflection recall rewiring to adapter
- T203: explicit memory tool + auto-capture rewiring to backend routes
- T204: `/new` and `/reset` reflection trigger rewiring to async backend enqueue
- T205: preserve actor-bound ownership semantics in the adapter

Verification guidance:
- use the stable backend toolchain path when backend verification is needed:
  - `~/.cargo/bin/cargo`
  - `~/.cargo/bin/rustc`
  - `CARGO_BUILD_JOBS=1`
  - `-j 1`
- reach at least `check` or `test` layer if meaningful code lands.

If meaningful implementation lands, update:
- docs/remote-memory-backend/task-plans/4phases-checklist.md

Preferred report:
- status
- changed files
- T201 outcome
- which of T202-T205 progressed
- verification results
- blockers / next continuation if unfinished
