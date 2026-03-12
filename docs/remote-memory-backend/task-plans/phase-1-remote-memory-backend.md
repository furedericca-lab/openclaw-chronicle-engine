---
description: Planning and contract-freeze tasks for remote-memory-backend.
---

# Tasks: remote-memory-backend

## Input

- `docs/remote-memory-backend/remote-memory-backend-brainstorming.md`
- `docs/remote-memory-backend/remote-memory-backend-implementation-research-notes.md`
- `docs/remote-memory-backend/remote-memory-backend-scope-milestones.md`
- `docs/remote-memory-backend/technical-documentation.md`
- `docs/remote-memory-backend/remote-memory-backend-contracts.md`
- `index.ts`
- `src/context/*`
- `src/store.ts`
- `src/embedder.ts`
- `src/retriever.ts`
- `src/scopes.ts`
- `src/reflection-store.ts`
- `src/tools.ts`

## Canonical architecture / Key constraints

- Remote backend is the only authority for ACL, scope, model config, gateway config, retrieval, and reflection execution.
- Local shell keeps OpenClaw hook/tool wiring plus local `src/context/*`.
- Local shell must not send requested scopes or provider config.
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

Checkpoint: documentation is concrete enough to begin backend and shell implementation without changing authority semantics later.

## Dependencies & Execution Order

- Phase 1 blocks all later phases.
- Tasks marked `[P]` may run concurrently only if they do not touch the same files.
