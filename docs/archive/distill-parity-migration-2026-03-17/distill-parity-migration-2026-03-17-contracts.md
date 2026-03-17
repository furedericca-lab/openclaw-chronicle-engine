---
description: Contracts for the distill parity migration scope.
---

# Contracts

## API Contracts

### `POST /v1/session-transcripts/append`

Purpose:

- persist ordered transcript rows for a caller-scoped session source.

Request:

- authenticated runtime caller;
- `actor.userId`, `actor.agentId`, `actor.sessionId`, `actor.sessionKey`;
- `items[]` non-empty with `role` in `user|assistant|system` and non-empty `text`.

Response `200`:

- `{ "appended": <number> }`

Contract notes:

- backend assigns sequence order;
- request must include idempotency key;
- this route is backend-owned transcript persistence, not ordinary memory extraction.

### `POST /v1/distill/jobs`

Purpose:

- enqueue backend-native distill work over either inline input or a persisted transcript source.

Supported source kinds:

- `inline-messages`
- `session-transcript`

`session-transcript` request contract:

- `sessionKey` required;
- `sessionId` optional but recommended when the caller wants an exact session slice;
- request is scoped by authenticated principal plus source session identity;
- request succeeds when transcript rows exist and fails with structured source-unavailable semantics only when the requested source is absent.

### `GET /v1/distill/jobs/{jobId}`

Returns:

- caller-scoped job state;
- `sourceKind`;
- result summary or structured error payload.

## Shared Types / Schema Ownership

- transcript storage owner: backend SQLite table `session_transcript_messages`;
- transcript row schema:
  - `user_id`
  - `agent_id`
  - `session_key`
  - `session_id`
  - `seq`
  - `role`
  - `text`
  - `created_at`
- artifact schema owner: backend `distill_artifacts` table plus `DistillArtifact` JSON/status payloads;
- plugin owns only typed forwarding code, not transcript-source truth.

## Validation Rules And Compatibility Policy

- actor principal must match authenticated request headers;
- transcript append requires non-empty `items[]`;
- distill still rejects invalid `persistMemoryRows` combinations for governance mode;
- reducer parity is deterministic and backend-native:
  - dedupe by normalized text
  - evidence required
  - vague advisory text without causal/action structure is rejected
  - final artifact fields remain stable and inspectable
- compatibility target is behavioral parity, not restoration of sidecar topology.

## Security-Sensitive Fields And Boundaries

- `userId`, `agentId`, `sessionId`, and `sessionKey` are authority-bearing routing fields and must stay caller-scoped;
- transcript persistence must not be inferred from local files, queue directories, or example workers;
- backend remains the only supported authority for transcript-source resolution, job lifecycle, artifact persistence, and optional memory-row persistence.

## Cleanup And Compatibility Closeout

- removed residue:
  - `scripts/jsonl_distill.py`
  - `test/jsonl-distill-slash-filter.test.mjs`
  - `examples/new-session-distill/*`
- archived docs remain audit history only and must not be treated as active runtime guidance.
