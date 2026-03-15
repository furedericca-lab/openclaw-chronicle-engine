---
description: Derived execution handoff for Codex to start Phase 2 backend implementation without reopening frozen contract decisions.
---

# Phase 2 Implementation Handoff: remote-memory-backend

## Status

This is a derived handoff summary for execution only.
Canonical planning and acceptance remain in:

- `docs/remote-memory-backend/remote-memory-backend-contracts.md`
- `docs/remote-memory-backend/technical-documentation.md`
- `docs/remote-memory-backend/remote-memory-backend-scope-milestones.md`
- `docs/remote-memory-backend/task-plans/phase-2-remote-memory-backend.md`
- `docs/remote-memory-backend/task-plans/4phases-checklist.md`

Do not reopen frozen contract points unless implementation reveals a hard contradiction.

## Goal

Start Phase 2 by standing up the backend-side MVP contract surface and its contract tests from the already-frozen docs.

Primary objective for this Codex run:

- inspect the repo and current implementation state;
- identify the concrete Phase 2 code shape and file plan;
- implement the smallest credible first batch toward T101-T105;
- add or update tests that lock the newly frozen runtime semantics;
- update the Phase 2 checklist/checkpoint evidence if meaningful code lands.

## Frozen contract points that must not drift

1. `POST /v1/memories/store` supports exactly two request shapes via `mode`:
   - `tool-store`
   - `auto-capture`
2. `tool-store` preserves explicit `category` and `importance`.
3. Ordinary runtime write/update payloads must not accept scope from the shell.
4. `POST /v1/memories/update` exists as a dedicated endpoint.
5. Data-plane stats route is `POST /v1/memories/stats`, not a GET/query contract.
6. Reflection job status on the data plane is caller-scoped by `(userId, agentId)` principal.
7. Operator-global inspection belongs only to admin routes.
8. `POST /v1/memories/list` uses:
   - frozen category enum
   - default `createdAt DESC`
   - `nextOffset: null` on the final page
9. `sessionKey` is stable logical provenance; `sessionId` is ephemeral diagnostics only.
10. Stable recall DTOs must not expose raw vector/BM25/rerank breakdown internals.
11. Remote MVP parity is intentionally limited; deferred CLI/operator commands must not scope-creep into Phase 2.

## Expected Phase 2 implementation scope

Implement or scaffold the backend-side shape for:

- `GET /v1/health`
- `POST /v1/recall/generic`
- `POST /v1/recall/reflection`
- `POST /v1/memories/store`
- `POST /v1/memories/update`
- `POST /v1/memories/delete`
- `POST /v1/memories/list`
- `POST /v1/memories/stats`
- `POST /v1/reflection/jobs`
- `GET /v1/reflection/jobs/{jobId}`

Also enforce or scaffold:

- auth/token class separation
- backend-owned ACL/scope derivation
- SQLite-backed reflection job ownership/status
- contract validation for frozen request/response semantics

## Deferred from this phase unless trivially required

Do not expand scope into:

- `delete-bulk`
- `export`
- `import`
- `reembed`
- migration CLI parity
- FTS maintenance/telemetry endpoints
- admin mutation/config endpoints
- rich scoring-debug runtime DTOs

## Recommended execution approach

1. Inspect the current repo to determine whether a Rust backend skeleton already exists.
2. If there is no backend skeleton yet, land the smallest coherent starter structure for T101 plus contract-focused tests or placeholders.
3. If backend scaffolding exists, prioritize contract-surface correctness over broad feature expansion.
4. Prefer implementing validation and test locks early for:
   - `tool-store` vs `auto-capture`
   - forbidden `scope`
   - category enum validation
   - `POST /v1/memories/stats`
   - caller-scoped job status
   - `nextOffset=null`
5. Keep changes auditable and update `docs/remote-memory-backend/task-plans/4phases-checklist.md` if a meaningful Phase 2 batch completes.

## Verification target for this run

Aim to reach at least `test` layer if code lands.

Minimum useful verification:

- repository tests covering the new contract points; and/or
- build/check command for the introduced backend surface; and/or
- a documented blocked state with exact missing prerequisites.

## Output expectations for Codex

Report back with:

- status: `changed` / `passed` / `partial` / `blocked`
- changed files
- implemented endpoints or scaffolds
- tests added/updated
- verification command results
- blockers requiring a new continuation if not done
