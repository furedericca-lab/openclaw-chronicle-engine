---
description: Planning and contract-freeze tasks for remote-memory-backend.
---

# Tasks: remote-memory-backend

## Input

- `docs/remote-memory-backend/remote-memory-backend-brainstorming.md`
- `docs/remote-memory-backend/remote-memory-backend-implementation-research-notes.md`
- `docs/remote-memory-backend/remote-memory-backend-scope-milestones.md`
- `docs/remote-memory-backend-2026-03-17/remote-memory-backend-2026-03-17-technical-documentation.md`
- `docs/remote-memory-backend/remote-memory-backend-contracts.md`
- `index.ts`
- `src/context/*`
- `src/store.ts`
- `src/embedder.ts`
- `src/retriever.ts`
- `src/scopes.ts`
- `src/reflection-store.ts`
- `src/tools.ts`
- `cli.ts`

## Canonical architecture / Key constraints

- Remote backend is the only authority for ACL, scope, model config, gateway config, retrieval, and reflection execution.
- Local shell keeps OpenClaw hook/tool wiring plus local `src/context/*`.
- Local shell must send actor identity plus operation payloads only; it must not send requested scopes, scope overrides, or provider config.
- No local fallback backend behavior is allowed.
- Phase 1 is docs and contract freeze only; no code migration yet.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 1: Authority and API freeze
Goal: freeze the authority model, contract surface, and migration boundaries before implementation.
Definition of Done: backend/shell/orchestration boundaries and endpoint contracts are explicit enough that implementation can start without rediscovery.

Tasks:
- [ ] T001 [Docs] Finalize phased documentation for the remote backend scope.
  - DoD: `docs/remote-memory-backend/*.md` and `task-plans/*.md` exist with concrete repo paths, endpoint names, and migration boundaries.
- [ ] T002 [P] [Security] Lock singular-authority rules for ACL, scope, config, and fallback behavior.
  - DoD: contracts and technical docs explicitly forbid local scope authority and local fallback backend behavior.
- [ ] T003 [P] [QA] Define validation coverage for backend contracts and shell behavior.
  - DoD: docs include concrete verification commands and expected runtime behaviors for recall failure, write failure, and async reflection enqueue.
- [ ] T004 [Docs] Freeze `memory_store` request-shape semantics and explicit field ownership.
  - DoD: `tool-store` and `auto-capture` request bodies are both documented, `category` / `importance` behavior is explicit, and scope is forbidden in ordinary runtime writes.
- [ ] T005 [Docs] Freeze the dedicated `memory_update` endpoint.
  - DoD: endpoint name, request shape, allowed patch fields, and response semantics are documented and consistent with current tool parity goals.
- [ ] T006 [Security] Resolve stats and reflection-job authority boundaries.
  - DoD: data-plane stats uses the actor-envelope model consistently, reflection job visibility is caller-scoped by principal, and admin-token inspection is clearly separated into admin routes.
- [ ] T007 [P] [Docs] Freeze list pagination/order and actor identity semantics.
  - DoD: default ordering, `nextOffset` terminal behavior, frozen category enum, and `sessionKey` vs `sessionId` responsibilities are explicit in contract docs.
- [ ] T008 [P] [Docs] Freeze DTO exposure and MVP parity boundaries.
  - DoD: recall DTOs are explicitly decoupled from raw scoring internals, and deferred CLI/operator capabilities are documented as out of MVP remote parity.

Checkpoint: documentation is concrete enough to begin backend and shell implementation without changing authority semantics later.

## Dependencies & Execution Order

- Phase 1 blocks all later phases.
- Tasks marked `[P]` may run concurrently only if they do not touch the same files.
