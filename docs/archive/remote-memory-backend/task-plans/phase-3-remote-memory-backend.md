---
description: Local shell integration tasks for remote-memory-backend.
---

# Tasks: remote-memory-backend

## Input

- `docs/remote-memory-backend/phase-2-sign-off-note.md`
- `docs/remote-memory-backend/remote-memory-backend-contracts.md`
- `docs/remote-memory-backend-2026-03-17/remote-memory-backend-2026-03-17-technical-documentation.md`
- backend MVP implementation from phase 2
- `index.ts`
- `src/context/*`
- current local backend modules under `src/*.ts`

## Canonical architecture / Key constraints

- `src/context/*` stays local.
- adapter layer should be thin and transport-focused.
- shell must send actor identity plus operation payloads only; it must not send provider config, requested scopes, or scope overrides.
- shell must not keep fallback backend behavior.
- `/new` and `/reset` must enqueue reflection jobs asynchronously and return immediately.
- Phase 3 main integration work is gated by a precondition verification batch because Phase 2 sign-off accepted remaining verification blockers explicitly.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 3: Thin shell adapter integration
Goal: replace local backend authority with a thin HTTP-backed adapter while preserving local orchestration semantics.
Definition of Done: OpenClaw lifecycle and tool flows call the remote backend through a thin adapter, while local `src/context/*` and session-local state remain local.

## Phase 3 precondition gate (must complete before T201-T205)
Goal: obtain a credible verification stamp for the Phase 2 backend in a more stable execution path before starting shell integration.
Definition of Done: repo docs contain a trustworthy verification record for the current backend patch, or an explicit environment-blocked note detailed enough to justify delaying main integration.

Precondition tasks:
- [x] T3P01 [QA] Re-run backend verification using a more stable command path.
  - DoD: `cargo check` completes from the backend worktree using a command/environment combination chosen to reduce prior linker/outer-run instability, and the exact command is recorded.
- [x] T3P02 [QA] Run targeted backend tests for Phase 2 contract-critical paths.
  - DoD: targeted tests covering auth-context binding, LanceDB persistence, idempotency lifecycle, and reflection job ownership complete with recorded results.
- [x] T3P03 [Infra] Avoid prior outer-run interruption patterns.
  - DoD: the verification execution path avoids the earlier outer `SIGTERM` failure mode and documents any remaining environment limits.
- [x] T3P04 [Docs] Record the verification stamp or explicit blocker note before main integration starts.
  - DoD: `docs/remote-memory-backend/task-plans/4phases-checklist.md` captures commands, results, and whether Phase 3 main tasks are cleared to begin.

Main integration tasks (start only after T3P01-T3P04 are satisfied):
- [x] T201 [Backend] Introduce a local REST adapter layer for backend communication.
  - DoD: transport/auth/retry logic is isolated from `src/context/*` and `index.ts` no longer reaches local backend primitives directly.
- [x] T202 [P] [Agentic] Rewire generic recall and reflection recall planning to consume backend-returned rows.
  - DoD: local orchestration uses the adapter instead of local authority-bearing backend modules and keeps prompt rendering/session-local behavior unchanged.
- [x] T203 [P] [Agentic] Rewire explicit memory tool and auto-capture flows to use backend store/update/delete/list/stats routes.
  - DoD: tool semantics match the frozen runtime contracts, including `tool-store` vs `auto-capture`, and explicit failures surface to callers.
- [x] T204 [Infra] Rewire `/new` and `/reset` reflection triggers to async backend job enqueue.
  - DoD: reflection trigger paths no longer execute reflection locally and do not block dialogue on job completion.
- [x] T205 [P] [Security] Preserve actor-bound ownership semantics in the shell adapter.
  - DoD: shell passes `sessionKey` and `sessionId` with their frozen responsibilities, does not rebuild local scope authority, and does not depend on operator-only job visibility.

## Phase 3 closeout evidence (2026-03-13)

- Added focused remote-mode shell verification under `test/remote-backend-shell-integration.test.mjs` covering:
  - remote `memory_recall` / `memory_store` / `memory_forget` / `memory_update` registration + route behavior
  - `agent_end` auto-capture forwarding via `mode=auto-capture`
  - remote reflection recall path in `before_prompt_build`
  - async, non-blocking `/new` + `/reset` reflection enqueue path
  - runtime context preservation for `userId`, `agentId`, `sessionId`, `sessionKey`
  - explicit proof that remote tool/request payloads do not include local scope authority fields
- Verification commands:
  - `node --test --test-name-pattern='.' test/remote-backend-shell-integration.test.mjs`
  - `npm test`
- Result:
  - all remote closeout tests passed
  - full repo test suite passed

Checkpoint: local shell is transport-only for memory authority concerns, while `src/context/*` remains local and session-local.

## Dependencies & Execution Order

- Phase 3 depends on Phase 2.
- `T3P01-T3P04` are mandatory preconditions before `T201-T205`.
- `T201` should start before `T202-T205`.
- Tasks marked `[P]` may run concurrently only if they do not touch the same files.
