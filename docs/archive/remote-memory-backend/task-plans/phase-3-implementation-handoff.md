---
description: Derived execution handoff for Codex to start Phase 3 main integration after the precondition verification gate was cleared.
---

# Phase 3 Implementation Handoff: remote-memory-backend

## Status

This is a derived execution handoff.
Canonical planning and acceptance remain in:

- `docs/remote-memory-backend/phase-2-sign-off-note.md`
- `docs/remote-memory-backend/remote-memory-backend-contracts.md`
- `docs/remote-memory-backend-2026-03-17/remote-memory-backend-2026-03-17-technical-documentation.md`
- `docs/remote-memory-backend/task-plans/phase-3-remote-memory-backend.md`
- `docs/remote-memory-backend/task-plans/4phases-checklist.md`

The Phase 3 precondition gate is complete.
Main integration tasks `T201-T205` are now cleared to start.

## Goal

Start Phase 3 main integration by replacing local backend authority with a thin HTTP-backed shell adapter while preserving local `src/context/*` orchestration and session-local behavior.

Execution order for this run:

1. **T201 first** — establish the local REST adapter layer and transport boundary.
2. **Then drive T202-T205 in parallel where practical** — only after the adapter seam is real and the call sites can be rewired against it.

## Mandatory sequencing rule

Do not start broad call-site rewiring before T201 is materially landed.

Minimum bar for T201 to count as started:

- a local backend client / adapter seam exists;
- transport/auth/retry concerns are no longer smeared directly across `index.ts` and orchestration call sites;
- there is a clear place for actor-envelope construction and trusted runtime identity header forwarding.

## Frozen constraints you must preserve

- `src/context/*` remains local and keeps prompt-time gating, rendering, and session-local state.
- Shell remains thin; it must not regain local ACL/scope authority.
- Shell sends actor identity plus operation payloads only; no requested scopes, scope overrides, or provider config.
- Runtime contract remains:
  - `POST /v1/memories/store` with `tool-store` / `auto-capture`
  - `POST /v1/memories/update`
  - `POST /v1/memories/delete`
  - `POST /v1/memories/list`
  - `POST /v1/memories/stats`
  - `POST /v1/recall/generic`
  - `POST /v1/recall/reflection`
  - `POST /v1/reflection/jobs`
  - `GET /v1/reflection/jobs/{jobId}`
- `sessionKey` remains stable logical provenance; `sessionId` remains ephemeral diagnostics only.
- Caller-scoped ownership must remain intact.
- `/new` and `/reset` remain non-blocking and must enqueue reflection jobs asynchronously.
- Do not expand into deferred CLI/operator surfaces.

## Expected implementation focus by task

### T201 — local REST adapter layer (must come first)
Suggested implementation shape:

- introduce local adapter/client modules, for example under `src/backend-client/` or a similarly named focused location;
- centralize:
  - base URL/config loading
  - bearer token handling
  - trusted runtime identity header forwarding
  - actor-envelope request shaping
  - retry/error translation
  - endpoint-specific client methods
- make the adapter the only route through which shell-side memory authority interactions happen.

### T202 — generic recall + reflection recall rewiring
- move orchestration callers onto the adapter methods;
- preserve existing local prompt logic and suppression behavior;
- do not let raw HTTP details leak into `src/context/*`.

### T203 — explicit memory tool + auto-capture rewiring
- rewire tool flows to backend store/update/delete/list/stats routes;
- preserve frozen `tool-store` vs `auto-capture` semantics;
- surface explicit failures to callers.

### T204 — `/new` and `/reset` reflection trigger rewiring
- remove local reflection execution on those paths;
- enqueue backend jobs asynchronously;
- preserve non-blocking dialogue behavior.

### T205 — actor-bound ownership semantics in adapter
- ensure shell forwards `sessionKey` and `sessionId` with their frozen roles;
- do not depend on operator-only visibility;
- do not reconstruct local scope authority.

## Verification target for this run

Try to reach at least `check` or `test` layer if meaningful code lands.

Preferred verification shape:

- adapter-focused unit tests where practical;
- existing repo tests updated or added for rewired code paths;
- at minimum, concrete commands proving the new adapter seam compiles and the touched paths are wired consistently.

Use the stable toolchain path established by the precondition gate when backend verification is required:

- `~/.cargo/bin/cargo`
- `~/.cargo/bin/rustc`
- prefer serialized builds when needed:
  - `CARGO_BUILD_JOBS=1`
  - `-j 1`

## Documentation update requirement

If meaningful implementation lands, update:

- `docs/remote-memory-backend/task-plans/4phases-checklist.md`

The checklist should record:
- which of `T201-T205` moved
- evidence commands
- blockers if T202-T205 could not progress after T201

## Preferred report back

- status
- changed files
- T201 outcome
- which of T202-T205 progressed
- verification results
- blockers / next continuation if not done
