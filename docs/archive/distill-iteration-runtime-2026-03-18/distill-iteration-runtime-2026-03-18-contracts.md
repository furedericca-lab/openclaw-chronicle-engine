---
description: API and schema contracts for distill-iteration-runtime-2026-03-18.
---

# distill-iteration-runtime-2026-03-18 Contracts

## API Contracts

- Existing backend distill job routes remain canonical:
  - `POST /v1/distill/jobs`
  - `GET /v1/distill/jobs/{jobId}`
- Runtime automatic distill may only enqueue through the existing backend job API.

## Shared Types / Schemas

- Plugin config gains an optional `distill` section with:
  - `enabled`
  - `mode`
  - `persistMode`
  - `everyTurns`
  - `maxMessages`
  - `maxArtifacts`
  - `chunkChars`
  - `chunkOverlapMessages`
- `everyTurns` means completed user sends in a caller-scoped session.

## Ownership and Compatibility

- Backend remains transcript source and artifact/memory persistence authority.
- Runtime owns only cadence bookkeeping and enqueue timing.
- Existing manual distill tools remain valid and unchanged.

## Event and Streaming Contracts

- `agent_end` remains the automatic trigger point.
- Automatic distill cadence advances only when the transcript batch contains one or more `user` rows.
- One user send counts as one turn.
- Automatic distill enqueue must use a stable idempotency key.

## Error Model

- Transcript append remains fail-open.
- Automatic distill enqueue remains fail-open and does not block session completion.
- Invalid distill numeric config values continue to be rejected by existing positive-integer parsing or backend request validation.

## Validation and Compatibility Rules

- New tests must prove:
  - multi-message evidence aggregation
  - structured English summary output
  - cadence-based automatic enqueue every N user turns
  - assistant-only batches do not advance cadence

## Rejected Historical Shapes / Non-Goals

- No language-adaptive extraction.
- No external sidecar worker.
- No backend model-backed map phase.
