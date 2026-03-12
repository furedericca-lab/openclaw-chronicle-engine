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

Required headers for authenticated endpoints:

```text
Authorization: Bearer <token>
X-Request-Id: <uuid>
```

Required additional header for write endpoints and reflection-job enqueue:

```text
Idempotency-Key: <stable-unique-key>
```

Token classes:

- `user token`: normal shell/tool traffic; ACL enforced
- `admin token`: management traffic; ACL bypass allowed only on admin endpoints

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

Field ownership:

- shell provides actor identity only;
- backend derives scope and ACL internally;
- shell does not provide scope hints or provider config.

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
    "sessionId": "agent:main:uuid",
    "sessionKey": "agent:main:uuid"
  },
  "query": "user preferences about editor",
  "channel": "auto-recall",
  "limit": 3
}
```

Request rules:

- `query`: required non-empty string
- `channel`: enum: `auto-recall`, `manual-search`
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

Status model:

- `200`, `400`, `401`, `403`, `429`, `500`, `503`

### `POST /v1/memories/store`

Purpose:
- explicit tool writes and auto-capture writes

Request:

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

Request rules:

- `mode`: enum: `auto-capture`, `tool-store`
- `items`: non-empty array
- `role`: enum: `user`, `assistant`, `system`
- `text`: required string

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

Action enum:

- `ADD`
- `UPDATE`
- `DELETE`
- `NOOP`

Status model:

- `200`, `400`, `401`, `403`, `409`, `429`, `500`, `503`

### `POST /v1/memories/delete`

Request by id:

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

Request by query:

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
    "sessionId": "agent:main:uuid",
    "sessionKey": "agent:main:uuid"
  },
  "limit": 50,
  "offset": 0,
  "category": "preference"
}
```

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

Status model:

- `200`, `400`, `401`, `403`, `500`

### `GET /v1/memories/stats`

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

Status model:

- `200`, `400`, `401`, `403`, `500`

Auth rule:

- admin token may bypass ACL.

### `POST /v1/reflection/jobs`

Purpose:
- enqueue async reflection generation on `/new` and `/reset`

Request:

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

Trigger enum:

- `new`
- `reset`

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

## Validation rules and compatibility policy

Validation rules:

- all write and job-enqueue requests require `Idempotency-Key`
- actor envelope required on all non-health, non-admin-free endpoints
- backend must reject invalid combinations such as both `memoryId` and `query` on delete
- backend must clamp oversized limits instead of trusting raw client values

Compatibility policy:

- shell must stay thin and not regain local scope authority;
- no dual-authority fallback path is allowed during migration;
- local `src/context/*` remains local even after backend migration.

## Security-sensitive fields and redaction / masking requirements

- TOML config secrets must never appear in REST responses.
- upstream provider error details must be summarized before returning in `details`.
- admin-token-only endpoints must be explicit and auditable.
- scope names may be returned for observability, but not accepted as client authority input.
