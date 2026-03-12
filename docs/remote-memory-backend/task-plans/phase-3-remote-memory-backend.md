---
description: Local shell integration tasks for remote-memory-backend.
---

# Tasks: remote-memory-backend

## Input

- `docs/remote-memory-backend/remote-memory-backend-contracts.md`
- `docs/remote-memory-backend/technical-documentation.md`
- backend MVP implementation from phase 2
- `index.ts`
- `src/context/*`
- current local backend modules under `src/*.ts`

## Canonical architecture / Key constraints

- `src/context/*` stays local.
- adapter layer should be thin and transport-focused.
- shell must not send provider config or requested scopes.
- shell must not keep fallback backend behavior.
- `/new` and `/reset` must enqueue reflection jobs asynchronously and return immediately.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 3: Thin shell adapter integration
Goal: replace local backend authority with a thin HTTP-backed adapter while preserving local orchestration semantics.
Definition of Done: OpenClaw lifecycle and tool flows call the remote backend through a thin adapter, while local `src/context/*` and session-local state remain local.

Tasks:
- [ ] T201 [Backend] Introduce a local REST adapter layer for backend communication.
  - DoD: transport/auth/retry logic is isolated from `src/context/*` and `index.ts` no longer reaches local backend primitives directly.
- [ ] T202 [P] [Agentic] Rewire generic recall and reflection recall planning to consume backend-returned rows.
  - DoD: local orchestration uses the adapter instead of local scope-aware backend modules and keeps prompt rendering/session-local behavior unchanged.
- [ ] T203 [P] [Agentic] Rewire explicit memory tool and auto-capture flows to use backend store/delete/list/stats routes.
  - DoD: tool semantics match the new backend result contracts and explicit failures surface to callers.
- [ ] T204 [Infra] Rewire `/new` and `/reset` reflection triggers to async backend job enqueue.
  - DoD: reflection trigger paths no longer execute reflection locally and do not block dialogue on job completion.

Checkpoint: local shell is transport-only for memory authority concerns, while `src/context/*` remains local and session-local.

## Dependencies & Execution Order

- Phase 3 depends on Phase 2.
- `T201` should start before `T202-T204`.
- Tasks marked `[P]` may run concurrently only if they do not touch the same files.
