---
description: API and schema contracts for the remote Rust memory backend MVP.
---

# remote-memory-backend Contracts

## API contracts

### Transport and authentication

Base URL example:

```text
http://memory-backend.internal:8080
```

Required headers for authenticated data-plane endpoints:

```text
Authorization: Bearer <token>
X-Request-Id: <uuid>
X-Auth-User-Id: <gateway-authenticated-user-id>
X-Auth-Agent-Id: <gateway-authenticated-agent-id>
```

Required additional header for write endpoints and reflection-job enqueue:

```text
Idempotency-Key: <stable-unique-key>
```

Runtime identity handoff rule:

- `X-Auth-User-Id` and `X-Auth-Agent-Id` are trusted principal headers injected by the gateway/runtime auth layer after bearer validation;
- shell/runtime request bodies still carry the actor envelope for request semantics, but principal authorization is bound to the trusted headers above;
- shell must not synthesize `userId` / `agentId` from static fallback config when runtime identity is missing;
- shell/runtime must not recover `agentId` from `sessionKey`; `sessionKey` is not a principal source;
- when runtime principal identity is unavailable:
  - recall routes are skipped fail-open with warning logs;
  - write/update/delete/list/stats/reflection-job-enqueue routes are blocked fail-closed with explicit errors;
- for direct backend integration tests, callers must provide both trusted identity headers explicitly.

Token classes:

- `user token`: ordinary shell/context/tool traffic on the data plane; ACL and ownership enforced
- `admin token`: control-plane management traffic; may bypass ordinary actor ACL on admin endpoints, but never bypasses audit requirements

### Plane separation

Contract rules:

- ordinary runtime endpoints are data-plane interfaces for shell/context/tool usage;
- admin endpoints are a separate control plane for operators and debugging;
- ordinary runtime contracts must not expose or depend on admin capabilities;
- user tokens are for data-plane operations only;
- admin-token authority is valid only on explicitly marked admin/control-plane routes.

### Shared request schema

All non-health data-plane request bodies use this actor envelope:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "runtime-instance-uuid",
    "sessionKey": "agent:main:uuid"
  }
}
```

Field ownership:

- shell provides actor envelope fields only;
- gateway/runtime auth layer provides trusted principal headers (`X-Auth-User-Id`, `X-Auth-Agent-Id`);
- backend derives scope and ACL internally;
- shell does not provide scope hints or provider config.

Actor field responsibilities:

- `userId`: principal owner of the request;
- `agentId`: agent-level principal boundary for ACL and job ownership;
- `sessionKey`: stable OpenClaw conversational/session identity used for provenance, audit correlation, and async job context (not principal derivation);
- `sessionId`: ephemeral runtime execution instance identifier used for request tracing and diagnostics only.

Authority rules:

- backend authorization and logical ownership must not depend on `sessionId` stability;
- if `sessionId` changes across retries or runtime restarts, data-plane ownership remains tied to principal identity plus `sessionKey` semantics;
- backend may record `sessionId` for audit and observability, but must not require it for long-lived visibility guarantees.
- shell may generate `sessionId` for diagnostics when absent, but must not fabricate data-plane ownership principals (`userId`, `agentId`).
- shell/runtime must treat missing explicit `agentId` as missing principal identity even when `sessionKey` is present.
- backend must reject actor envelopes whose `(userId, agentId)` do not match trusted runtime identity headers.

Admin/control-plane note:

- admin endpoints do not use the ordinary actor envelope as their primary authority model;
- admin authority comes from the admin token plus explicit audit context.

### Shared error schema

All non-2xx responses return:

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

MVP error codes:

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

### Frozen category enum

The MVP category set is frozen to:

- `preference`
- `fact`
- `decision`
- `entity`
- `reflection`
- `other`

Contract rule:

- data-plane endpoints that accept `category` must treat the set above as the only valid `/v1` category enum values;
- unknown category values must return `400 INVALID_REQUEST`.

### `GET /v1/health`

Response `200`:

```json
{
  "status": "ok",
  "service": "memory-backend",
  "version": "0.1.0"
}
```

### `POST /v1/recall/generic`

Purpose:
- auto-recall and explicit search-like retrieval

Request:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "runtime-instance-uuid",
    "sessionKey": "agent:main:uuid"
  },
  "query": "user preferences about editor",
  "limit": 3
}
```

Request rules:

- `query`: required non-empty string
- `limit`: required positive integer; backend may clamp

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
      "metadata": {
        "createdAt": 1741770000000,
        "updatedAt": 1741773600000
      }
    }
  ]
}
```

Contract rules:

- `score` is a stable ranking output field, not a promise of raw backend scoring internals;
- detailed score-breakdown fields such as vector/BM25/rerank contributions are not part of the stable `/v1` runtime DTO;
- backend may expose richer diagnostic scoring only outside the ordinary stable data-plane contract.

2026-03-17 refresh note:

- backend now exposes debug-scoped trace routes outside the ordinary recall DTO contract:
  - `POST /v1/debug/recall/generic`
  - `POST /v1/debug/recall/reflection`
- these routes return structured retrieval trace data for debugging/verification while preserving the stable `/v1/recall/*` row schema.

Status model:

- `200`: success, including empty rows
- `400`, `401`, `403`, `429`, `500`, `503`

### `POST /v1/recall/reflection`

Purpose:
- backend-selected reflection rows for `<inherited-rules>`

Request:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "runtime-instance-uuid",
    "sessionKey": "agent:main:uuid"
  },
  "query": "current task prompt",
  "mode": "invariant+derived",
  "limit": 6
}
```

Request rules:

- `query`: required non-empty string
- `mode`: optional enum: `invariant-only`, `invariant+derived`
- `mode` default: `invariant+derived`
- `limit`: required positive integer; backend may clamp

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

Contract rules:

- runtime reflection DTOs must stay oriented around prompt-time orchestration semantics;
- detailed backend scoring breakdown is not part of the stable `/v1` reflection DTO.

Status model:

- `200`, `400`, `401`, `403`, `429`, `500`, `503`

### `POST /v1/memories/store`

Purpose:
- explicit tool writes and auto-capture writes

Contract rule:

- this endpoint supports two frozen request shapes selected by `mode`;
- `tool-store` and `auto-capture` are both part of MVP;
- shell must not provide scope in either shape.

#### Request shape: `mode = "tool-store"`

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "runtime-instance-uuid",
    "sessionKey": "agent:main:uuid"
  },
  "mode": "tool-store",
  "memory": {
    "text": "User prefers Neovim",
    "category": "preference",
    "importance": 0.82
  }
}
```

Tool-store request rules:

- `memory.text`: required non-empty string
- `memory.category`: optional frozen category enum; backend defaults to `other` if omitted
- `memory.importance`: optional number `0..1`; backend applies policy default if omitted
- `scope`: forbidden in tool-store request payloads

#### Request shape: `mode = "auto-capture"`

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "runtime-instance-uuid",
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

Auto-capture request rules:

- `items`: required non-empty array
- `role`: enum: `user`, `assistant`, `system`
- `text`: required non-empty string
- top-level `category` and `importance` are not accepted in auto-capture mode
- backend owns extraction, dedupe, classification, and target-scope selection

Response `200`:

```json
{
  "results": [
    {
      "id": "mem_x",
      "action": "ADD",
      "text": "User prefers Neovim",
      "category": "preference",
      "importance": 0.82,
      "scope": "agent:main"
    }
  ]
}
```

Action enum:

- `ADD`
- `UPDATE`
- `DELETE`
- `NOOP`

Status model:

- `200`, `400`, `401`, `403`, `409`, `429`, `500`, `503`

### `POST /v1/memories/update`

Purpose:
- explicit in-place update for an existing memory while preserving backend authority over ACL and scope

Request:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "runtime-instance-uuid",
    "sessionKey": "agent:main:uuid"
  },
  "memoryId": "mem_x",
  "patch": {
    "text": "User prefers Neovim for terminal editing",
    "category": "preference",
    "importance": 0.9
  }
}
```

Request rules:

- `memoryId`: required
- `patch`: required object containing at least one allowed field
- allowed patch fields:
  - `text`
  - `category`
  - `importance`
- `scope`: forbidden in update payloads
- backend owns ACL checks, target row resolution, and any required re-embedding when `text` changes

Response `200`:

```json
{
  "result": {
    "id": "mem_x",
    "action": "UPDATE",
    "text": "User prefers Neovim for terminal editing",
    "category": "preference",
    "importance": 0.9,
    "scope": "agent:main"
  }
}
```

Status model:

- `200`, `400`, `401`, `403`, `404`, `409`, `429`, `500`, `503`

### `POST /v1/memories/delete`

Request by id:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "runtime-instance-uuid",
    "sessionKey": "agent:main:uuid"
  },
  "memoryId": "mem_x"
}
```

Request by query:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "runtime-instance-uuid",
    "sessionKey": "agent:main:uuid"
  },
  "query": "user prefers vim"
}
```

Validation:

- exactly one of `memoryId` or `query`

Response `200`:

```json
{
  "deleted": 1
}
```

Status model:

- `200`, `400`, `401`, `403`, `404`, `409`, `500`

### `POST /v1/memories/list`

Request:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "runtime-instance-uuid",
    "sessionKey": "agent:main:uuid"
  },
  "limit": 50,
  "offset": 0,
  "category": "preference"
}
```

Request rules:

- `limit`: required positive integer; backend may clamp
- `offset`: required non-negative integer
- `category`: optional frozen category enum

Ordering and pagination rules:

- default ordering is `createdAt DESC`
- ties must use a deterministic secondary key
- `nextOffset` returns the next integer offset when more rows exist
- `nextOffset` is `null` when the returned page is the last page

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
        "createdAt": 1741770000000,
        "updatedAt": 1741773600000
      }
    }
  ],
  "nextOffset": 50
}
```

Last-page example:

```json
{
  "rows": [],
  "nextOffset": null
}
```

Status model:

- `200`, `400`, `401`, `403`, `500`

### `POST /v1/memories/stats`

Purpose:
- data-plane content statistics for the caller-visible memory surface

Request:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "runtime-instance-uuid",
    "sessionKey": "agent:main:uuid"
  }
}
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

Status model:

- `200`, `400`, `401`, `403`, `500`

Contract rules:

- this is a data-plane content-stats endpoint only;
- it uses the same actor-envelope authority model as other non-health data-plane endpoints;
- it returns only caller-visible content counts;
- it does not include provider health, job timing, or other operator-only management fields.

### Initial admin surface

The initial frozen control-plane surface is limited to:

- `GET /v1/admin/health`
- `GET /v1/admin/jobs`
- `GET /v1/admin/jobs/{jobId}`

Optional read-only extension within the same v1 control-plane scope:

- `GET /v1/admin/stats`

Explicitly not part of the initial frozen surface:

- admin config writes
- policy writes
- admin memory mutation endpoints
- bulk operator endpoints

Contract rules:

- admin endpoints are not part of the ordinary shell/context/tool runtime contract;
- admin endpoints may use admin-token authority instead of ordinary actor identity;
- admin endpoints are read-only in the initial frozen surface;
- admin endpoints must remain auditable.

### Admin audit minimum

All admin requests must emit an audit record containing at least:

- `timestamp`
- `requestId`
- `operatorId`
- `endpoint`
- `method`
- `target selector`
- `resultStatus`
- `statusCode`

Additional audit requirements:

- admin mutations must additionally require and record a `reason`;
- if the request explicitly targets a concrete scope or job, the audit record should include the relevant selector such as `targetScope` or `jobId`;
- if an `Idempotency-Key` is present, it should be included in the audit record.

### `POST /v1/reflection/jobs`

Purpose:
- enqueue async reflection generation on `/new` and `/reset`

Request:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "runtime-instance-uuid",
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

Trigger enum:

- `new`
- `reset`

Ownership rules:

- backend records the job owner from actor identity at enqueue time;
- the data-plane ownership principal is the `(userId, agentId)` pair;
- backend should additionally record `sessionKey` for audit and provenance;
- backend must not make data-plane job visibility depend on `sessionId` stability.

Response `202`:

```json
{
  "jobId": "job_x",
  "status": "queued"
}
```

Status model:

- `202`, `400`, `401`, `403`, `409`, `500`, `503`

### `GET /v1/reflection/jobs/{jobId}`

Purpose:
- data-plane diagnostics for reflection jobs owned by the caller principal

Visibility and token rules:

- user-token callers may view only jobs owned by the same `(userId, agentId)` principal;
- user-token access must not reveal jobs belonging to other principals even if a job id is guessed;
- admin-token inspection of arbitrary jobs belongs on admin endpoints such as `GET /v1/admin/jobs/{jobId}`;
- this ordinary data-plane route remains caller-scoped, not operator-global.

Response queued/running:

```json
{
  "jobId": "job_x",
  "status": "running"
}
```

Response completed:

```json
{
  "jobId": "job_x",
  "status": "completed",
  "persisted": true,
  "memoryCount": 4
}
```

Response failed:

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

Contract rules:

- response fields must remain minimal and diagnostics-oriented;
- provider internals, raw prompts, and operator-only debugging state are not part of this route.

Status model:

- `200`, `401`, `403`, `404`, `500`

## Shared types / schema definitions and ownership

Ownership rules:

- backend owns:
  - memory row schema
  - reflection row schema
  - scope and ACL derivation
  - job schema
  - provider/gateway config schema
- local shell owns:
  - mapping backend DTOs into local orchestration-friendly types
  - hook/tool error handling behavior
- `src/context/*` owns:
  - prompt block planning and rendering
  - session-local state and suppression

Compatibility rule:

- backend contracts are new and may diverge from local TypeScript internal row shapes;
- response DTO redesign is allowed only when it reduces ambiguity or avoids leaking backend internals without benefit.

## Event / async contracts

There is no SSE contract in MVP.

Async contract in MVP:

- shell enqueues reflection via `POST /v1/reflection/jobs`
- backend persists job state in SQLite
- shell may poll via `GET /v1/reflection/jobs/{jobId}` for diagnostics only
- main dialogue path must not depend on polling completion
- global operator inspection belongs to admin routes, not caller-scoped data-plane status routes

## Validation rules and compatibility policy

Validation rules:

- all write and job-enqueue requests require `Idempotency-Key`
- actor envelope required on all non-health, non-admin-free data-plane endpoints
- trusted runtime identity headers (`X-Auth-User-Id`, `X-Auth-Agent-Id`) are required on data-plane routes and must match actor principal fields
- shell mode-validation boundary:
  - `remoteBackend.enabled=true` requires `remoteBackend.baseURL` and `remoteBackend.authToken`, and may omit local `embedding`;
  - local mode (`remoteBackend.enabled=false` or unset) requires `embedding` and must fail at config-parse time when missing.
- backend must reject invalid combinations such as both `memoryId` and `query` on delete
- backend must clamp oversized limits instead of trusting raw client values
- `scope` is forbidden in ordinary runtime write/update payloads
- `category` must use the frozen `/v1` enum set above
- `POST /v1/memories/stats` is the canonical data-plane stats route for MVP
- idempotency records use explicit lifecycle states:
  - `reserved` -> `in_progress` -> `completed` on success
  - `reserved` -> `in_progress` -> `failed` when the protected side effect fails before completion
- failed idempotency records may be retried with the same key only when payload fingerprint matches
- completed idempotency records currently return `409 IDEMPOTENCY_CONFLICT` for repeated requests because full response replay is still deferred in MVP

Compatibility policy:

- `/v1` permits backward-compatible additive changes only;
- existing request/response fields and semantics must remain stable within `/v1`;
- breaking schema or semantic changes require a new major API version such as `/v2`;
- shell must stay thin and not regain local scope authority;
- no dual-authority fallback path is allowed during migration;
- local `src/context/*` remains local even after backend migration.

## Security-sensitive fields and redaction / masking requirements

- TOML config secrets must never appear in REST responses.
- upstream provider error details must be summarized before returning in `details`.
- admin-token-only endpoints must be explicit and auditable.
- scope names may be returned for observability, but not accepted as client authority input.
- data-plane job status must never expose operator-global job visibility.
