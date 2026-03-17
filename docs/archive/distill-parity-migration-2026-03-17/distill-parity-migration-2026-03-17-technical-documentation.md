---
description: Technical documentation for the distill parity migration execution scope.
---

# Technical Documentation

## Canonical Architecture

The supported runtime path is now:

1. runtime finishes a turn and emits `agent_end` in [index.ts](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/index.ts);
2. the plugin forwards ordered transcript rows to `POST /v1/session-transcripts/append`;
3. backend persists those rows in SQLite `session_transcript_messages`;
4. caller enqueues a distill job with `source.kind = session-transcript` or `inline-messages`;
5. backend resolves source rows, cleans/noise-filters them, applies deterministic reducer parity, persists artifacts, and optionally persists memory rows;
6. caller polls `GET /v1/distill/jobs/{jobId}`.

Primary modules:

- runtime wiring: [index.ts](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/index.ts)
- typed backend client: [src/backend-client/client.ts](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/src/backend-client/client.ts)
- HTTP contracts: [backend/src/lib.rs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/src/lib.rs)
- backend models/contracts: [backend/src/models.rs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/src/models.rs)
- transcript persistence, distill execution, reducer logic: [backend/src/state.rs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/src/state.rs)

## Key Constraints And Non-goals

- transcript authority is backend-owned; local files do not define supported transcript sources;
- ordinary auto-capture memory extraction remains distinct from transcript persistence;
- `session-transcript` must remain caller-scoped by authenticated `userId` + `agentId`;
- queue-file inboxes, systemd workers, and `memory-pro import` are rejected runtime shapes;
- reducer parity is behavioral and deterministic, not provider-topology parity.

## Interfaces Between Components

### Runtime -> backend transcript persistence

Route:

- `POST /v1/session-transcripts/append`

Payload:

- `actor`
- ordered `items[]` with `role` and `text`

Ownership:

- plugin forwards runtime transcript rows;
- backend assigns sequence ordering and persists rows.

### Distill job family

Routes:

- `POST /v1/distill/jobs`
- `GET /v1/distill/jobs/{jobId}`

Supported sources:

- `inline-messages`
- `session-transcript`

Reducer output:

- stable `category`, `importance`, `text`, `evidence`, `tags`, `persistence`

## Operational Behavior

### Transcript append

- registered at `agent_end`;
- fail-open on transport/backend failure;
- uses deterministic idempotency key derived from session identity + append batch;
- independent from `autoCapture`, so transcript persistence stays available even when memory extraction is disabled.

### Session-transcript distill

- loads caller-scoped transcript rows ordered by `seq`;
- optional `sessionId` narrows the source;
- `maxMessages` trims to the latest tail window;
- cleaned rows reuse the same transcript cleanup path as `inline-messages`.

### Deterministic reducer

- windows prepared rows by char budget and overlap;
- normalizes candidate text, tags, evidence, importance, and category;
- drops vague/low-signal rows without structured lesson content;
- dedupes on normalized text key;
- ranks artifacts by structured operational signal and evidence strength.

## Observability And Error Handling

- transcript append remains observable in plugin logs and backend idempotency records;
- missing transcript rows produce structured distill failure with source-unavailable semantics;
- ordinary backend errors still surface through existing job-status error envelopes;
- transcript append and auto-capture are logged separately so operators can distinguish source persistence from memory extraction.

## Security Model And Hardening Notes

- authenticated runtime principal headers must match `actor.userId` and `actor.agentId`;
- transcript rows are stored only under the caller principal + session identity;
- the plugin never claims transcript authority beyond forwarding runtime rows;
- removed sidecar artifacts must not be restored as alternate persistence or ACL paths.

## Test Strategy Mapping

Backend contract/integration:

- `cargo test --test phase2_contract_semantics distill_ -- --nocapture`

Plugin/runtime integration:

- `node --test test/remote-backend-shell-integration.test.mjs`

Documentation hygiene:

- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/distill-parity-migration-2026-03-17`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/distill-parity-migration-2026-03-17 README.md`
