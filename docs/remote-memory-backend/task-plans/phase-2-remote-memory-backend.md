---
description: Backend MVP implementation tasks for remote-memory-backend.
---

# Tasks: remote-memory-backend

## Input

- `docs/remote-memory-backend/remote-memory-backend-contracts.md`
- `docs/remote-memory-backend/technical-documentation.md`
- `docs/remote-memory-backend/remote-memory-backend-scope-milestones.md`

## Canonical architecture / Key constraints

- Backend runtime is Rust.
- Storage is LanceDB.
- Reflection job queue/status uses SQLite.
- Backend config is static TOML only.
- REST is the MVP transport.
- Backend must own ACL and scope derivation from phase 2 onward.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 2: Backend service MVP
Goal: stand up the Rust backend skeleton and the minimum backend-owned data/control paths.
Definition of Done: the backend can boot from TOML, answer health checks, enforce auth, and serve the agreed MVP memory/reflection endpoints against LanceDB and SQLite.

Tasks:
- [ ] T101 [Backend] Create the Rust service skeleton with config loading, auth middleware, and health route.
  - DoD: backend boots from TOML and serves `GET /v1/health` with auth behavior matching the contract.
- [ ] T102 [P] [Backend] Implement LanceDB-backed memory CRUD and recall endpoints.
  - DoD: `POST /v1/recall/generic`, `POST /v1/recall/reflection`, `POST /v1/memories/store`, `POST /v1/memories/delete`, `POST /v1/memories/list`, and `GET /v1/memories/stats` exist with schema-accurate responses.
- [ ] T103 [P] [Backend] Implement SQLite-backed reflection job enqueue/status paths.
  - DoD: `POST /v1/reflection/jobs` and `GET /v1/reflection/jobs/{jobId}` persist and report job lifecycle states through SQLite.
- [ ] T104 [Security] Implement backend-owned ACL and scope derivation.
  - DoD: no endpoint relies on client-supplied scope hints; ACL and scope decisions are made inside the backend only.

Checkpoint: the backend exists as a standalone MVP service and exposes the contract surface needed by the local shell.

## Dependencies & Execution Order

- Phase 2 depends on Phase 1.
- `T101` should land before or alongside `T102` and `T103`.
- `T104` must be complete before shell integration starts.
- Tasks marked `[P]` may run concurrently only if they do not touch the same files.
