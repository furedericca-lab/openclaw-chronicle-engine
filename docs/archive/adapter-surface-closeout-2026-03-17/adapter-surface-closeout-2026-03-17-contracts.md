---
description: API and schema contracts for adapter-surface-closeout-2026-03-17.
---

# adapter-surface-closeout-2026-03-17 Contracts

## API Contracts

This scope freezes adapter/plugin-facing contracts over already-existing backend routes. It does not redefine backend authority.

### Management-gated distill enqueue tool

Proposed public shell surface:

- tool name: `memory_distill_enqueue`
- availability: only when `enableManagementTools=true`
- authority: runtime principal required; fail closed when missing

Request parameters:

- `mode`: `session-lessons` | `governance-candidates`
- `sourceKind`: `inline-messages` | `session-transcript`
- `persistMode`: `artifacts-only` | `persist-memory-rows`
- `messages[]` required when `sourceKind=inline-messages`
- `sessionKey` required when `sourceKind=session-transcript`
- `sessionId` optional when `sourceKind=session-transcript`
- optional bounded execution options:
  - `maxMessages`
  - `chunkChars`
  - `chunkOverlapMessages`
  - `maxArtifacts`

Response:

- success:
  - `jobId`
  - `status`
  - echoed high-level source summary
- failure:
  - structured backend error envelope surfaced through the existing tool error pattern

Contract rules:

- plugin does not invent local distill jobs;
- plugin does not read transcript files as a substitute for backend transcript source;
- plugin does not accept scope overrides.

### Management-gated distill status tool

Proposed public shell surface:

- tool name: `memory_distill_status`
- availability: only when `enableManagementTools=true`
- request:
  - `jobId`
- response:
  - `jobId`
  - `status`
  - `mode`
  - `sourceKind`
  - `createdAt`
  - `updatedAt`
  - optional result summary or structured error payload

### Management-gated recall debug trace tool

Proposed public shell surface:

- tool name: `memory_recall_debug`
- availability: only when `enableManagementTools=true`
- authority: runtime principal required; fail closed when missing

Request parameters:

- `channel`: `generic` | `reflection`
- `query`
- `limit`
- `reflectionMode`:
  - only valid when `channel=reflection`
  - `invariant-only` | `invariant+derived`

Response:

- `rows[]` using the backend debug route row shape;
- `trace` using the backend debug route trace shape;
- plugin may summarize trace text for tool output, but must keep the raw trace in `details` for inspectability.

Contract rules:

- ordinary `memory_recall` remains unchanged;
- debug trace is not returned from ordinary recall by default;
- caller does not choose scope, ACL, or backend ranking strategy.

### Phase 1 frozen management-surface decisions

- distill is exposed as two tools, not one action-multiplexed tool:
  - `memory_distill_enqueue`
  - `memory_distill_status`
- debug recall trace is exposed as a separate management/debug tool:
  - `memory_recall_debug`
- all three tools stay behind `enableManagementTools=true`;
- all three tools require runtime principal identity and use the existing backend-client error normalization path;
- none of these tools may accept a client-provided `scope` override;
- ordinary `memory_recall` remains a non-debug surface and does not gain an inline debug flag in this scope.

## Shared types / schema definitions and ownership

- backend-owned DTOs:
  - ordinary recall rows
  - debug recall trace payloads
  - distill enqueue/status DTOs
- adapter-owned DTOs:
  - tool parameter schemas
  - tool output shaping for OpenClaw
  - startup/config deprecation warnings
- prompt-local ownership:
  - `setwise-v2` post-selection only over ordinary recall row data already returned by the backend

### `memoryReflection` config compatibility contract

Selected contract for this scope:

- `injectMode`, `messageCount`, `errorReminderMaxEntries`, `dedupeErrorSignals`, and `recall.*` remain supported prompt/orchestration config.
- `agentId`, `maxInputChars`, `timeoutMs`, and `thinkLevel` remain parseable in this scope for backward-compatibility only.
- the adapter must treat `agentId`, `maxInputChars`, `timeoutMs`, and `thinkLevel` as deprecated/ignored fields and must not use them to control runtime behavior.
- schema text, README text, and startup/config diagnostics must label those fields as deprecated/ignored rather than supported runtime knobs.
- removal of those deprecated fields is deferred until a later cleanup scope after docs and tests prove that no supported runtime path depends on them.

### `setwise-v2` runtime input contract

- supported production inputs:
  - normalized text key
  - ordinary recall score
  - category
  - scope
  - timestamp
- unsupported production assumptions:
  - backend-returned embeddings on ordinary recall routes
  - backend-returned BM25/rerank trace internals on ordinary recall routes

## Event contracts where applicable

- `agent_end` remains the only transcript append hook in the supported runtime path.
- `/new` and `/reset` remain reflection enqueue hooks only.
- this scope must not add a hidden automatic distill trigger to `agent_end` unless explicitly documented and tested as a new behavior.

## Error Model

- transport and backend failures continue to use the existing `MemoryBackendClientError` normalization path in the adapter layer;
- ordinary tool outputs must preserve the current split between:
  - missing runtime principal
  - remote backend error
  - generic transport/runtime error
- new distill/debug management tools must not invent local status codes or local retry semantics beyond the existing client wrapper;
- debug trace requests must fail as explicit management-tool errors rather than silently degrading into ordinary recall output.

## Validation rules and compatibility policy

- `enableManagementTools=false` must keep the current ordinary runtime/tool surface unchanged.
- new management tools must require runtime principal identity and use the existing remote-backend error mapping.
- ordinary recall/injection paths remain fail open.
- write/update/delete/list/stats/distill/debug management paths remain fail closed on missing runtime identity.
- no production path may depend on local reflection generation helpers after cleanup.
- any config-field removal must come with:
  - schema updates
  - README updates
  - release-note or migration-note evidence

## Security-sensitive fields and redaction/masking requirements

- `userId`, `agentId`, `sessionId`, and `sessionKey` remain authority-bearing inputs and must be sourced from the runtime context, not synthesized from static fallback config;
- debug trace outputs must stay caller-scoped and must not leak another principal's data;
- distill tool outputs must not dump unbounded transcript content by default;
- no new tool may accept a client-provided `scope` field;
- cleanup must preserve any transcript redaction/filtering still used by supported runtime flows.
