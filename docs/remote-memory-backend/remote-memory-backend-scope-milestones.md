---
description: Scope boundaries and milestones for the remote Rust memory backend migration.
---

# remote-memory-backend Scope and Milestones

## In scope

- Define the remote backend authority model and REST contracts.
- Define the local shell vs local orchestration boundary.
- Define the backend-owned ACL/scope/config model.
- Define the MVP reflection async-job model.
- Define phased implementation milestones for migrating from local TypeScript backend modules to a remote Rust service.
- Freeze the MVP runtime parity boundary between the current local CLI/tool surface and the required remote API surface.

## Out of scope

- Multi-node backend clustering or distributed workers.
- Broker-based queues or external orchestration systems.
- Environment-variable based backend config.
- Local fallback backend behavior.
- Reworking prompt tag semantics owned by `src/context/*`.
- Shipping the full migration implementation in this documentation batch.
- Full remote parity for all existing CLI/operator commands.

## Milestones

### Milestone 1 — Contract and authority freeze

Acceptance gate:

- backend authority boundaries are explicit and singular;
- phased docs describe what leaves the shell and what stays local;
- REST surface is concrete enough for backend and shell implementation planning;
- the following frozen contract points are explicit and implementation-ready:
  - `tool-store` request shape and `category` / `importance` behavior;
  - dedicated `memory_update` endpoint;
  - `POST /v1/memories/stats` actor/auth model;
  - reflection job ownership, visibility, and token boundary;
  - list ordering, pagination, and frozen category enum semantics;
  - `sessionKey` vs `sessionId` responsibilities;
  - recall DTO boundary against overexposed scoring internals;
  - MVP parity boundary vs deferred CLI/operator capabilities.

### Milestone 2 — Backend service MVP

Acceptance gate:

- Rust service skeleton exists with health/auth/config loading;
- LanceDB-backed storage and retrieval routes exist for the agreed MVP endpoints;
- SQLite reflection job table exists with enqueue/status paths;
- caller-scoped stats and job-ownership visibility rules are enforced in the backend.

### Milestone 3 — Local shell adapter integration

Acceptance gate:

- local shell no longer constructs local storage/retrieval/scope authority objects;
- `src/context/*` stays local but reads backend-returned rows through a thin adapter;
- `/new` and `/reset` trigger async reflection jobs without blocking OpenClaw dialogue;
- local tool wiring reflects the frozen `store/update/delete/list/stats` runtime contract without reintroducing local scope authority.

### Milestone 4 — Verification, migration safety, and operator readiness

Acceptance gate:

- contract tests and local shell behavior tests pass;
- failure semantics are verified;
- migration/rollback path is documented;
- admin-token management paths are either implemented or explicitly deferred with contract reservation;
- deferred CLI/operator surfaces are documented so MVP completion is not blocked by parity creep.

## Dependencies

- Milestone 1 blocks all later work.
- Milestone 2 depends on Milestone 1 contracts and technical docs.
- Milestone 3 depends on Milestone 2 backend routes existing.
- Milestone 4 depends on Milestones 2-3 because it validates the actual integrated system.

## Exit criteria

- the remote backend is documented as the single memory authority;
- the shell/backend boundary is concrete enough to implement without rediscovering semantics;
- the migration plan preserves local `src/context/*` ownership of prompt-time state and rendering;
- no mixed-authority scope or ACL path remains in the target design;
- MVP-required remote parity is explicit, and deferred CLI/operator features are clearly marked non-blocking for the first remote cut.
