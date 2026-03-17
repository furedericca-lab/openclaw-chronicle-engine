---
description: API and schema contracts for distill-backend-scope.
---

# distill-backend-scope Contracts

## API Contracts

This scope now ships backend-native distill enqueue/status plus the initial `inline-messages` executor slice, and freezes the remaining direction:

- distill capability is exposed as backend-native runtime endpoints for enqueue/status;
- the sidecar queue-file plus `memory-pro import` flow is not an accepted future authority model;
- distill enqueue/status follows the same actor-principal discipline as reflection jobs.
- old sidecar distill residue must be treated as cleanup debt with an explicit disposition, not as an alternate supported runtime path.

Current shipped public shape:

- `POST /v1/distill/jobs`
- `GET /v1/distill/jobs/{jobId}`

Frozen follow-up direction:

- first implementation should ship exactly one async distill job family;
- distill should not multiplex onto reflection endpoints;
- distill should use explicit request/response DTOs rather than implicit transcript-import side effects.
- `inline-messages` execution, artifact persistence, and optional memory-row persistence are shipped in the current batch.
- `session-transcript` source resolution and richer provider-driven extraction remain follow-up phases.

### Proposed `POST /v1/distill/jobs`

Purpose:

- enqueue a backend-native async transcript distill job for lesson extraction or governance-oriented summarization

Request:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "runtime-instance-uuid",
    "sessionKey": "agent:main:uuid"
  },
  "mode": "session-lessons",
  "source": {
    "kind": "session-transcript",
    "sessionKey": "agent:main:uuid",
    "sessionId": "runtime-instance-uuid"
  },
  "options": {
    "maxMessages": 400,
    "chunkChars": 12000,
    "chunkOverlapMessages": 10,
    "maxArtifacts": 20,
    "persistMode": "artifacts-only"
  }
}
```

Frozen request rules:

- `mode` initial enum is:
  - `session-lessons`
  - `governance-candidates`
- `source.kind` initial enum is:
  - `session-transcript`
  - `inline-messages`
- `session-transcript` source requires:
  - `sessionKey`
  - optional `sessionId`
- `inline-messages` source requires:
  - non-empty `messages[]`
- `options.persistMode` initial enum is:
  - `artifacts-only`
  - `persist-memory-rows`
- `persist-memory-rows` is allowed only for `mode = "session-lessons"` in the initial design.

Response `202`:

```json
{
  "jobId": "distill_job_x",
  "status": "queued"
}
```

Current shipped behavior:

- accepted requests persist a caller-scoped `distill_jobs` row with `status = "queued"`;
- backend now asynchronously executes `inline-messages` requests to `running -> completed|failed`;
- `session-transcript` requests are accepted into the job family but currently terminate with a structured source-unavailable failure.

### Proposed `GET /v1/distill/jobs/{jobId}`

Purpose:

- caller-scoped diagnostic/status retrieval for async distill jobs

Queued/running response:

```json
{
  "jobId": "distill_job_x",
  "status": "running",
  "mode": "session-lessons",
  "sourceKind": "session-transcript",
  "createdAt": 1772870400000,
  "updatedAt": 1772870465000
}
```

Completed response:

```json
{
  "jobId": "distill_job_x",
  "status": "completed",
  "mode": "session-lessons",
  "sourceKind": "session-transcript",
  "createdAt": 1772870400000,
  "updatedAt": 1772870465000,
  "result": {
    "artifactCount": 7,
    "persistedMemoryCount": 4,
    "warnings": []
  }
}
```

Failed response:

```json
{
  "jobId": "distill_job_x",
  "status": "failed",
  "mode": "session-lessons",
  "sourceKind": "session-transcript",
  "createdAt": 1772870400000,
  "updatedAt": 1772870465000,
  "error": {
    "code": "UPSTREAM_DISTILL_ERROR",
    "message": "Distill provider call failed",
    "retryable": false,
    "details": {}
  }
}
```

## Shared Types / Schemas

Planned conceptual schema families:

- `DistillJobRequest`
  - `actor`
  - `mode`
  - `source`
  - `options`
- `DistillJobStatus`
  - `jobId`
  - `status`
  - `mode`
  - `sourceKind`
  - `createdAt`
  - `updatedAt`
  - optional `result`
  - optional `error`
- `DistillArtifact`
  - stable artifact payload
  - evidence references
  - optional persistence mapping summary

Frozen initial DTO shapes:

### `DistillJobRequest`

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "runtime-instance-uuid",
    "sessionKey": "agent:main:uuid"
  },
  "mode": "session-lessons",
  "source": {
    "kind": "session-transcript",
    "sessionKey": "agent:main:uuid",
    "sessionId": "runtime-instance-uuid",
    "messages": []
  },
  "options": {
    "maxMessages": 400,
    "chunkChars": 12000,
    "chunkOverlapMessages": 10,
    "maxArtifacts": 20,
    "persistMode": "artifacts-only"
  }
}
```

### `DistillJobStatus`

```json
{
  "jobId": "distill_job_x",
  "status": "completed",
  "mode": "session-lessons",
  "sourceKind": "session-transcript",
  "createdAt": 1772870400000,
  "updatedAt": 1772870465000,
  "result": {
    "artifactCount": 7,
    "persistedMemoryCount": 4,
    "warnings": []
  }
}
```

### `DistillArtifact`

```json
{
  "artifactId": "art_x",
  "jobId": "distill_job_x",
  "kind": "lesson",
  "category": "fact",
  "importance": 0.84,
  "text": "Pitfall: ... Cause: ... Fix: ... Prevention: ...",
  "evidence": [
    {
      "messageIds": [12, 13],
      "quote": "..."
    }
  ],
  "tags": ["openclaw", "restart", "timeout"],
  "persistence": {
    "persistMode": "persist-memory-rows",
    "persistedMemoryIds": ["mem_x"]
  }
}
```

Frozen initial artifact enums:

- `kind`:
  - `lesson`
  - `governance-candidate`
- `category`:
  - `fact`
  - `decision`
  - `preference`
  - `other`

Frozen initial job states:

- `queued`
- `running`
- `completed`
- `failed`

Frozen lifecycle rules:

- `queued -> running -> completed`
- `queued -> running -> failed`
- direct `queued -> completed` is not allowed;
- direct `queued -> failed` is not allowed;
- retries, if later supported, should create a new job record rather than mutating a completed/failed job back to `queued`.

Current implementation note:

- `queued`, `running`, `completed`, and `failed` are now all reachable in the shipped implementation;
- current successful execution is limited to `source.kind = "inline-messages"`;
- future work must preserve the frozen DTO/state model while expanding source resolution and extraction quality.

Contract rule:

- distill artifacts must not be treated as reflection rows by default;
- if distill eventually persists ordinary memory rows, that mapping must be explicit rather than implicit.
- the initial implementation should store artifacts separately from ordinary memory rows even when `persistMode = "persist-memory-rows"` is requested.

## Event and Streaming Contracts

- distill is an async job surface, not a synchronous prompt-time path;
- shell may enqueue and poll, but must not own result persistence;
- `/new` and `/reset` may trigger distill in future only if the flow remains non-blocking and separately observable from reflection jobs.
- until backend-native distill exists, existing sidecar/example artifacts remain reference-only and must not regain production-path status.

## Error Model

Future distill should inherit existing backend error discipline:

- enqueue failures are visible and structured;
- job execution failures persist in job status;
- caller-scoped status endpoints must not leak other principals' jobs;
- transcript-ingest failures must be distinguishable from provider-execution failures.

Frozen initial error codes:

- `INVALID_REQUEST`
- `UNAUTHORIZED`
- `FORBIDDEN`
- `NOT_FOUND`
- `BACKEND_UNAVAILABLE`
- `UPSTREAM_DISTILL_ERROR`
- `DISTILL_SOURCE_UNAVAILABLE`
- `DISTILL_VALIDATION_ERROR`
- `IDEMPOTENCY_CONFLICT`
- `INTERNAL_ERROR`

## Validation and Compatibility Rules

- current reflection and auto-capture contracts must remain unchanged while planning distill;
- docs must clearly state that `jsonl_distill.py` is not the canonical authority path;
- docs must record which current residue is temporary migration reference vs eventual archive/remove target;
- any implementation scope that starts from this plan must add focused tests before moving sidecar logic into backend code;
- remote backend docs must describe distill as a distinct future capability rather than leaving it implied or ambiguous.
- first implementation should use a dedicated `distill_jobs` table instead of reusing the reflection job table;
- first implementation should use a dedicated `distill_artifacts` table or equivalent persisted artifact store, not overload reflection rows.
