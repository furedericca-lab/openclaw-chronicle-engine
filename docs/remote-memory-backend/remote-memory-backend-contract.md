---
description: Contract draft for the Rust remote memory backend and thin local OpenClaw shell.
---

# remote-memory-backend Contract

## Context

`memory-lancedb-pro` currently keeps backend memory capabilities and OpenClaw integration in the same runtime. The selected redesign splits those responsibilities into:

- a remote `Rust + LanceDB` backend that becomes the only memory authority;
- a thin local OpenClaw integration shell that keeps hook/tool wiring and `src/context/*`;
- local `src/context/*` orchestration that still owns prompt gating, block rendering, and session-local suppression/dedupe state.

This contract defines the minimum stable REST surface for that split. It is an MVP contract, not a multi-node or multi-tenant design.

## Findings

- Current generic auto-recall orchestration expects a backend-facing retrieval function plus local scope resolution. See [src/context/auto-recall-orchestrator.ts](/root/verify/memory-lancedb-pro-context-engine-split/src/context/auto-recall-orchestrator.ts).
- Current reflection prompt orchestration expects backend-facing list/read access plus local scope resolution and local session state. See [src/context/reflection-prompt-planner.ts](/root/verify/memory-lancedb-pro-context-engine-split/src/context/reflection-prompt-planner.ts).
- Existing docs already separate backend memory capabilities from prompt-time orchestration, but they still assume backend modules are local TypeScript modules. See [docs/context-engine-split/technical-documentation.md](/root/verify/memory-lancedb-pro-context-engine-split/docs/context-engine-split/technical-documentation.md).
- The new design intentionally removes local scope authority. The backend must own scope derivation, ACL, model config, gateway config, reflection execution, and persistence.

## Goals / Non-goals

Goals:
- Define an MVP REST contract for a remote `Rust + LanceDB` backend.
- Make the backend the only authority for ACL, scope, storage, retrieval, ranking, reflection execution, and persistence.
- Keep the local OpenClaw side thin: hook binding, tool binding, HTTP retry, fail-open recall behavior, and `src/context/*`.
- Keep `/new` and `/reset` non-blocking by turning reflection generation into an async backend job.
- Make request/response/error shapes concrete enough for schema validation and backend/client implementation.

Non-goals:
- Multi-node backend clustering.
- Broker-based distributed queues.
- Local fallback backend behavior.
- Environment-variable driven config for the backend MVP.
- Full implementation planning in this document.

## Target files / modules

Local shell and orchestration modules that must eventually conform to this contract:

- [index.ts](/root/verify/memory-lancedb-pro-context-engine-split/index.ts)
- [src/context/auto-recall-orchestrator.ts](/root/verify/memory-lancedb-pro-context-engine-split/src/context/auto-recall-orchestrator.ts)
- [src/context/reflection-prompt-planner.ts](/root/verify/memory-lancedb-pro-context-engine-split/src/context/reflection-prompt-planner.ts)
- [src/context/session-exposure-state.ts](/root/verify/memory-lancedb-pro-context-engine-split/src/context/session-exposure-state.ts)
- [src/context/prompt-block-renderer.ts](/root/verify/memory-lancedb-pro-context-engine-split/src/context/prompt-block-renderer.ts)

Backend capabilities to be replaced by the remote service:

- [src/store.ts](/root/verify/memory-lancedb-pro-context-engine-split/src/store.ts)
- [src/embedder.ts](/root/verify/memory-lancedb-pro-context-engine-split/src/embedder.ts)
- [src/retriever.ts](/root/verify/memory-lancedb-pro-context-engine-split/src/retriever.ts)
- [src/scopes.ts](/root/verify/memory-lancedb-pro-context-engine-split/src/scopes.ts)
- [src/reflection-store.ts](/root/verify/memory-lancedb-pro-context-engine-split/src/reflection-store.ts)
- [src/tools.ts](/root/verify/memory-lancedb-pro-context-engine-split/src/tools.ts)

## Constraints

- Backend language/runtime: `Rust`.
- Backend storage engine: `LanceDB`.
- Reflection job queue and status tracking: `SQLite job table`.
- Backend configuration source for MVP: static `TOML` file only.
- Backend owns ACL, scope derivation, model config, gateway config, rerank config, and reflection config.
- Local shell must not send scope decisions or provider config to the backend.
- Local shell must not implement a fallback backend.
- Generic recall failure remains fail-open locally: OpenClaw conversation continues without injection.
- Explicit write/delete tool failures must still surface as errors to the user/tool caller.
- Management endpoints may bypass ACL only when called with an admin token.

## API Contract

### Transport and auth model

Base URL example:

```text
http://memory-backend.internal:8080
```

Required headers for all authenticated endpoints:

```text
Authorization: Bearer <token>
X-Request-Id: <uuid>
```

Required additional header for all write endpoints and async-job trigger endpoints:

```text
Idempotency-Key: <stable-unique-key>
```

Token classes:

- `user token`: normal shell/tool traffic; ACL enforced.
- `admin token`: operational traffic; may bypass ACL on explicitly marked management endpoints.

Content type:

```text
Content-Type: application/json
```

### Shared request schema

All non-health request bodies use this actor envelope:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "agent:main:uuid",
    "sessionKey": "agent:main:uuid"
  }
}
```

Actor field rules:

- `userId`: required, stable user identifier.
- `agentId`: required, logical OpenClaw agent identity.
- `sessionId`: required for turn-scoped recall/capture behavior.
- `sessionKey`: required when backend needs parity with OpenClaw session-key semantics, especially for reflection job metadata.

The local shell does not send `scope`, `requestedScopes`, ACL rules, model config, or gateway config.

### Shared error schema

All non-2xx responses must return:

```json
{
  "error": {
    "code": "BACKEND_UNAVAILABLE",
    "message": "Human-readable summary",
    "retryable": true,
    "details": {}
  }
}
```

Error field rules:

- `code`: stable machine-readable enum string.
- `message`: human-readable summary safe for logs and shell warnings.
- `retryable`: backend-declared retry hint for the shell.
- `details`: optional structured metadata. Must not contain secrets or full model payloads.

Canonical error codes for MVP:

- `UNAUTHORIZED`
- `FORBIDDEN`
- `INVALID_REQUEST`
- `NOT_FOUND`
- `CONFLICT`
- `BACKEND_UNAVAILABLE`
- `UPSTREAM_EMBEDDING_ERROR`
- `UPSTREAM_RERANK_ERROR`
- `UPSTREAM_REFLECTION_ERROR`
- `RATE_LIMITED`
- `IDEMPOTENCY_CONFLICT`
- `INTERNAL_ERROR`

### Endpoint: `GET /v1/health`

Purpose:
- liveness/readiness probe for shell bootstrap and operator checks.

Response `200`:

```json
{
  "status": "ok",
  "service": "memory-backend",
  "version": "0.1.0"
}
```

### Endpoint: `POST /v1/recall/generic`

Purpose:
- generic memory recall for auto-recall and explicit search-like use cases.
- backend performs ACL, scope resolution, retrieval, rerank, scoring, and final top-k selection.

Request schema:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "agent:main:uuid",
    "sessionKey": "agent:main:uuid"
  },
  "query": "user preferences about editor",
  "channel": "auto-recall",
  "limit": 3
}
```

Request field rules:

- `query`: required, non-empty string.
- `channel`: required enum.
  - allowed: `auto-recall`, `manual-search`
- `limit`: required positive integer, backend may clamp.

Response `200`:

```json
{
  "rows": [
    {
      "id": "mem_x",
      "text": "User prefers Neovim",
      "category": "preference",
      "scope": "agent:main",
      "score": 0.91,
      "sources": {
        "vector": 0.82,
        "bm25": 0.64,
        "reranked": 0.93
      },
      "metadata": {
        "createdAt": 1741770000000,
        "updatedAt": 1741773600000
      }
    }
  ]
}
```

Response field rules:

- `rows`: ordered final results already selected by the backend.
- `category`: enum:
  - `preference`, `fact`, `decision`, `entity`, `other`, `reflection`
- `scope`: backend-owned resolved storage namespace, exposed for observability only.
- `score`: normalized final ranking score.
- `sources`: optional scoring breakdown for observability and future debug UI.
- `metadata`: optional non-secret metadata safe for local rendering/debug.

Status codes:

- `200`: success, including empty `rows`.
- `400`: malformed request.
- `401`: missing/invalid token.
- `403`: ACL denies recall for actor.
- `429`: backend or upstream rate-limited.
- `500`: unexpected backend failure.
- `503`: backend dependency unavailable.

Local-shell behavior:

- on `5xx` or `503`, fail open and do not inject memory.
- on `401` or `403`, fail open and log a warning.

### Endpoint: `POST /v1/recall/reflection`

Purpose:
- fetch final reflection recall rows for `<inherited-rules>`.
- backend performs ACL, scope resolution, reflection-row loading, ranking, filtering, and final top-k selection.

Request schema:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "agent:main:uuid",
    "sessionKey": "agent:main:uuid"
  },
  "query": "current task prompt",
  "limit": 6
}
```

Response `200`:

```json
{
  "rows": [
    {
      "id": "refl_x",
      "text": "Verify service health before changing DNS",
      "kind": "invariant",
      "strictKey": "dns-health-check",
      "scope": "agent:main",
      "score": 0.88,
      "metadata": {
        "timestamp": 1741770000000
      }
    }
  ]
}
```

Field rules:

- `kind`: required enum:
  - `invariant`, `derived`
- `strictKey`: required stable grouping key for reflection semantics.

Status codes:

- same as `POST /v1/recall/generic`

Local-shell behavior:

- on failure, skip inherited-rules injection and continue prompt build.

### Endpoint: `POST /v1/memories/store`

Purpose:
- explicit tool writes and auto-capture writes.
- backend decides final scope, extraction, dedupe, ADD/UPDATE/DELETE/NOOP action, and persistence.

Request schema:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "agent:main:uuid",
    "sessionKey": "agent:main:uuid"
  },
  "mode": "auto-capture",
  "items": [
    {
      "role": "user",
      "text": "I use tmux daily"
    },
    {
      "role": "assistant",
      "text": "Noted"
    }
  ]
}
```

Request field rules:

- `mode`: required enum:
  - `auto-capture`, `tool-store`
- `items`: required non-empty array.
- `role`: required enum:
  - `user`, `assistant`, `system`
- `text`: required string.

Response `200`:

```json
{
  "results": [
    {
      "id": "mem_x",
      "action": "ADD",
      "text": "User uses tmux daily",
      "scope": "agent:main"
    }
  ]
}
```

Response field rules:

- `action`: required enum:
  - `ADD`, `UPDATE`, `DELETE`, `NOOP`

Status codes:

- `200`: success.
- `400`: malformed request.
- `401`: missing/invalid token.
- `403`: ACL denies write for actor.
- `409`: idempotency conflict.
- `429`: backend or upstream rate-limited.
- `500`: unexpected backend failure.
- `503`: upstream model/backend unavailable.

Local-shell behavior:

- explicit tool calls surface non-2xx as errors.
- auto-capture path logs warnings and continues conversation flow.

### Endpoint: `POST /v1/memories/delete`

Purpose:
- explicit forget/delete operations.

Request schema by id:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "agent:main:uuid",
    "sessionKey": "agent:main:uuid"
  },
  "memoryId": "mem_x"
}
```

Request schema by query:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "agent:main:uuid",
    "sessionKey": "agent:main:uuid"
  },
  "query": "user prefers vim"
}
```

Validation rules:

- exactly one of `memoryId` or `query` must be provided.

Response `200`:

```json
{
  "deleted": 1
}
```

Status codes:

- `200`, `400`, `401`, `403`, `404`, `409`, `500`

### Endpoint: `POST /v1/memories/list`

Purpose:
- list memories for tools or management views.

Request schema:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "agent:main:uuid",
    "sessionKey": "agent:main:uuid"
  },
  "limit": 50,
  "offset": 0,
  "category": "preference"
}
```

Request field rules:

- `limit`: required positive integer, backend may clamp.
- `offset`: required non-negative integer.
- `category`: optional enum from memory categories.

Response `200`:

```json
{
  "rows": [
    {
      "id": "mem_x",
      "text": "User prefers Neovim",
      "category": "preference",
      "scope": "agent:main",
      "metadata": {
        "createdAt": 1741770000000
      }
    }
  ],
  "nextOffset": 50
}
```

Status codes:

- `200`, `400`, `401`, `403`, `500`

### Endpoint: `GET /v1/memories/stats`

Purpose:
- stats for shell diagnostics, CLI, or operator views.

Query parameters:

```text
userId=<string>&agentId=<string>
```

Response `200`:

```json
{
  "memoryCount": 128,
  "reflectionCount": 14,
  "categories": {
    "preference": 30,
    "fact": 80,
    "reflection": 14
  }
}
```

Auth rules:

- user token: ACL enforced.
- admin token: ACL bypass allowed.

Status codes:

- `200`, `400`, `401`, `403`, `500`

### Endpoint: `POST /v1/reflection/jobs`

Purpose:
- enqueue async reflection generation on local `/new` or `/reset`.
- request must return quickly and never block the OpenClaw conversation flow.

Request schema:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "agent:main:uuid",
    "sessionKey": "agent:main:uuid"
  },
  "trigger": "reset",
  "messages": [
    {
      "role": "user",
      "text": "..."
    }
  ]
}
```

Request field rules:

- `trigger`: required enum:
  - `new`, `reset`
- `messages`: required array of transcript items used for reflection input.

Response `202`:

```json
{
  "jobId": "job_x",
  "status": "queued"
}
```

Status codes:

- `202`: accepted.
- `400`: malformed request.
- `401`: missing/invalid token.
- `403`: ACL denies reflection generation for actor.
- `409`: duplicate idempotency key/job conflict.
- `500`: backend error.
- `503`: reflection subsystem unavailable.

Local-shell behavior:

- enqueue failure logs warning and does not block `/new` or `/reset`.

### Endpoint: `GET /v1/reflection/jobs/{jobId}`

Purpose:
- poll or inspect async reflection job status.

Response `200` queued/running:

```json
{
  "jobId": "job_x",
  "status": "running"
}
```

Response `200` completed:

```json
{
  "jobId": "job_x",
  "status": "completed",
  "persisted": true,
  "memoryCount": 4
}
```

Response `200` failed:

```json
{
  "jobId": "job_x",
  "status": "failed",
  "error": {
    "code": "UPSTREAM_REFLECTION_ERROR",
    "message": "Reflection model call failed",
    "retryable": false,
    "details": {}
  }
}
```

Status codes:

- `200`, `401`, `403`, `404`, `500`

### Optional admin-only management endpoints

These endpoints are not required for the first shell integration pass, but the contract reserves them because admin-token ACL bypass is part of the agreed MVP.

- `POST /v1/admin/memories/list`
- `POST /v1/admin/memories/delete`
- `GET /v1/admin/memories/stats`
- `GET /v1/admin/reflection/jobs`

Rules:

- require admin token;
- may bypass actor ACL;
- must still emit the same response/error schema family;
- must not expose raw secrets or upstream payloads.

## Verification plan

Contract verification work for this document should include:

```bash
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/remote-memory-backend
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/remote-memory-backend README.md
```

Implementation-time verification expected from this contract:

- backend contract tests for each endpoint status model and schema;
- shell-side tests proving:
  - recall failures stay fail-open;
  - tool write/delete failures surface to callers;
  - `/new` and `/reset` do not wait for reflection completion;
  - local `src/context/*` no longer depends on local scope authority.

## Rollback

If the remote backend migration regresses runtime behavior:

- keep `src/context/*` local and intact;
- revert the shell to local backend module wiring;
- do not keep mixed-authority ACL or scope logic;
- do not enable partial local fallback backend behavior.

The rollback rule is simple: authority is singular. Either the backend is authoritative, or the local legacy path is authoritative, but never both at once.

## Open questions

- Should `GET /v1/memories/stats` be enough for the shell, or will the future CLI require a richer stats payload in MVP?
- Should the admin management endpoints be implemented in MVP phase 1, or only reserved in the contract and delivered in a later implementation phase?
- Should reflection job status keep only aggregate counts, or also expose backend-generated timestamps for operator diagnostics?

## Execution log / evidence updates

- 2026-03-12: created initial single-contract draft for remote `Rust + LanceDB` backend MVP.
- 2026-03-12: locked authority boundaries:
  - backend owns ACL, scope, provider config, reflection execution, persistence;
  - local shell owns OpenClaw integration and local orchestration state only.
- 2026-03-12: captured agreed operational choices:
  - REST transport;
  - SQLite job table for reflection queue/status;
  - static TOML config only for backend MVP;
  - no local fallback backend.
