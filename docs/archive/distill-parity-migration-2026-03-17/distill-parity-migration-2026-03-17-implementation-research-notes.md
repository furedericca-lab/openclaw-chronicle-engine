---
description: Research notes for finishing transcript-source and reducer parity migration for distill.
---

# Implementation Research Notes

## Problem Statement And Current Baseline

Before this scope landed, the repo had a split state:

- backend distill already shipped enqueue/status plus `inline-messages` execution in [backend/src/lib.rs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/src/lib.rs) and [backend/src/state.rs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/src/state.rs);
- `source.kind = session-transcript` existed in the API shape but failed during execution because the backend had no persisted transcript source;
- transcript ingest/filter behavior still lived in the removed `scripts/jsonl_distill.py`;
- reducer heuristics still lived in the removed `examples/new-session-distill/worker/lesson-extract-worker.mjs`.

The missing backend capability was not the DTO shape. It was the absence of persisted, caller-scoped transcript rows with role/order/session identity.

## Gap Analysis With Repo Evidence

### Gap 1. No persisted transcript source

- `StoreRequest::AutoCapture` only wrote ordinary memory rows into LanceDB and did not preserve `sessionId`, `sessionKey`, role, or order.
- SQLite job tables stored only reflection/distill metadata, not transcript messages.
- Result: backend could not reconstruct a real session transcript from its own data.

### Gap 2. Runtime did not forward transcript rows to a backend-owned source

- `index.ts` previously forwarded only `mode=auto-capture` writes at `agent_end`.
- That path intentionally filtered assistant content depending on config and was not a safe authority source for later transcript distill.

### Gap 3. Reducer parity was narrower than the old worker

The example worker contained the useful parity targets:

- transcript windowing by char budget and overlap;
- candidate normalization;
- exact-text dedupe;
- evidence gating;
- ranking that prefers structured fix/cause/action lessons.

The worker topology itself was explicitly out of scope.

## Architecture And Implementation Options

### Option A. Reuse `memories_v1` as transcript source

Rejected.

- loses role and order;
- mixes ordinary memory mutations with transcript authority;
- cannot safely reconstruct a caller-scoped session transcript.

### Option B. Extend `mode=auto-capture` to double as transcript storage

Rejected as the sole contract.

- would couple transcript authority to auto-capture filtering semantics;
- would make `captureAssistant=false` silently change transcript completeness.

### Option C. Add dedicated transcript persistence and reuse the existing distill pipeline

Selected.

- add a dedicated SQLite table for transcript rows;
- add `POST /v1/session-transcripts/append`;
- have `index.ts` forward `agent_end` transcript rows into that endpoint with a deterministic idempotency key;
- load persisted transcript rows for `source.kind = session-transcript`;
- run them through the existing cleanup/noise filter path, then a deterministic reducer.

This keeps transcript authority backend-owned without reviving any local sidecar behavior.

## Selected Design And Landed Changes

### 1. Backend-owned transcript persistence

Landed in:

- [backend/src/models.rs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/src/models.rs)
- [backend/src/lib.rs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/src/lib.rs)
- [backend/src/state.rs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/src/state.rs)

Design:

- new route: `POST /v1/session-transcripts/append`;
- request shape: caller actor + ordered `items`;
- storage: SQLite table `session_transcript_messages(user_id, agent_id, session_key, session_id, seq, role, text, created_at)`;
- idempotency: normal backend idempotency protection plus a deterministic client-side key derived from session identity and appended transcript batch.

### 2. Backend-native `session-transcript` execution

Landed in:

- [backend/src/state.rs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/src/state.rs)

Design:

- `session-transcript` now loads persisted transcript rows scoped by `user_id`, `agent_id`, `sessionKey`, and optional `sessionId`;
- `maxMessages` trims to the latest caller-scoped tail window;
- loaded rows reuse the same cleanup/noise filter path as `inline-messages`;
- requests still fail with structured source-unavailable semantics when the scoped transcript source is missing.

### 3. Deterministic reducer parity

Landed in:

- [backend/src/state.rs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/src/state.rs)

Preserved behavior:

- char-budget transcript windowing with overlap;
- candidate normalization and stable field shaping;
- duplicate suppression via normalized text key;
- evidence gating and low-signal/vague-advice filtering;
- ranking that favors structured operational lessons.

Rejected behavior:

- queue-file inboxes;
- provider-specific worker topology;
- `memory-pro import` writeback.

### 4. Runtime wiring

Landed in:

- [index.ts](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/index.ts)
- [src/backend-client/types.ts](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/src/backend-client/types.ts)
- [src/backend-client/client.ts](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/src/backend-client/client.ts)

Behavior:

- `agent_end` always forwards transcript rows to `POST /v1/session-transcripts/append`;
- ordinary auto-capture memory extraction remains separately controlled by `autoCapture`;
- transcript append is fail-open and does not block interaction.

### 5. Residue cleanup

Removed from the active repo runtime:

- `scripts/jsonl_distill.py`
- `test/jsonl-distill-slash-filter.test.mjs`
- `examples/new-session-distill/*`

## Test And Validation Strategy

Backend:

- `cargo test --test phase2_contract_semantics distill_ -- --nocapture`
Expected:
- `session-transcript` succeeds when transcript rows exist;
- `session-transcript` fails with structured source-unavailable semantics only when rows are absent;
- `inline-messages` still completes and persists correctly.

Plugin/runtime:

- `node --test test/remote-backend-shell-integration.test.mjs`
Expected:
- `agent_end` appends transcript rows;
- `autoCapture=false` still keeps transcript append active;
- remote routing and reflection tests remain green.

Documentation hygiene:

- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/distill-parity-migration-2026-03-17`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/distill-parity-migration-2026-03-17 README.md`

## Risks, Assumptions, Unresolved Questions

- Replay protection for transcript append currently relies on deterministic append idempotency keys derived from the batch payload. Exact repeated turns with identical payloads would coalesce; this is acceptable for the current non-sidecar parity target but should remain explicit.
- Transcript persistence currently happens on `agent_end`. If future distill use cases require mid-session execution, the same backend authority model should be extended to an earlier runtime event rather than reintroducing local file authority.
- The reducer is intentionally deterministic and narrower than the historical provider-backed worker. Future improvements should refine scoring/selection inside the backend, not restore the removed sidecar topology.
