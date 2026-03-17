---
description: API and schema contracts for memory-backend-gap-closeout-2026-03-17.
---

# memory-backend-gap-closeout-2026-03-17 Contracts

## API Contracts

### Reflection enqueue source contract

Selected direction:

- `/new` and `/reset` reflection enqueue must stop depending on plugin-local session file discovery and parsing;
- reflection input messages must come from a dedicated caller-scoped backend route:
  - `POST /v1/reflection/source`
- plugin may still normalize runtime event payloads, but it must not reconstruct historical session content from local disk in the supported runtime.

Request shape:

```json
{
  "actor": {
    "userId": "u_123",
    "agentId": "main",
    "sessionId": "runtime-instance-uuid",
    "sessionKey": "agent:main:uuid"
  },
  "trigger": "new",
  "maxMessages": 120
}
```

Response shape:

```json
{
  "messages": [
    { "role": "user", "text": "..." },
    { "role": "assistant", "text": "..." }
  ]
}
```

Contract rules:

- plugin does not read local OpenClaw session JSONL files as canonical reflection source;
- plugin does not scan `workspace/sessions` or agent session directories to recover previous conversation content for reflection;
- backend remains responsible for transcript storage semantics and any historical message selection used for reflection.
- backend changes are required in this scope because no public caller-scoped reflection-source route exists today.

### Reflection status shell surface

Selected public shell surface:

- tool name: `memory_reflection_status`
- availability: management-gated under `enableManagementTools=true`
- authority: runtime principal required; fail closed when missing

Request parameters:

- `jobId`

Response:

- `jobId`
- `status`
- optional `persisted`
- optional `memoryCount`
- optional structured error payload

Contract rules:

- status remains caller-scoped;
- tool does not accept scope overrides;
- tool does not expose operator-global job inspection.
- tool remains fail-closed when runtime principal identity is missing.

### Reflection enqueue/status adapter contract

- `src/backend-client/types.ts` must define stable DTOs for:
  - reflection status;
  - `POST /v1/reflection/source`.
- `src/backend-client/client.ts` must expose typed methods for those routes.
- `index.ts` must use only those adapter methods in supported runtime reflection flows.

## Shared Types / Schemas

- backend-owned:
  - transcript source-of-truth
  - reflection job lifecycle state
  - caller-visible reflection job status DTO
- adapter-owned:
  - tool parameter schema for `memory_reflection_status`
  - config warnings/errors for deprecated compatibility fields
- test/reference-only:
  - historical TS helpers that are not imported by supported runtime modules

## Ownership and Compatibility

- `sessionMemory.enabled` and `sessionMemory.messageCount` are legacy compatibility inputs only and must not obscure the current runtime model.
- `memoryReflection.agentId`, `maxInputChars`, `timeoutMs`, and `thinkLevel` remain warning-only compatibility fields unless this scope explicitly upgrades them to hard deprecation.
- prompt-local `adaptive-retrieval` and `setwise-v2` remain supported local seams and are not part of the backend closeout target.

## Event and Streaming Contracts

- `agent_end` remains the transcript append hook.
- `/new` and `/reset` remain reflection enqueue hooks.
- no local reflection generation fallback may be reintroduced.
- `POST /v1/reflection/source` remains caller-scoped and principal-bound.

## Error Model

- missing runtime principal:
  - recall remains fail-open where already defined;
  - reflection status and any reflection-source management/read route fail closed.
- backend transport/status errors:
  - continue to use `MemoryBackendClientError` normalization.
- unsupported legacy config:
  - at minimum warns clearly;
  - may hard-fail only if explicitly documented in this scope’s migration decision.

## Validation and Compatibility Rules

- no supported runtime path may import `src/query-expander.ts` or `src/reflection-store.ts`;
- any removal or relocation of test/reference helpers requires import-proof and test updates;
- README, README_CN, and plugin schema text must match the implemented runtime surface.

## Rejected Historical Shapes / Non-Goals

- local session-file reflection source recovery as supported runtime behavior;
- ungated reflection job inspection;
- local fallback memory authority;
- migration of prompt-local heuristics that do not recreate backend authority.
