---
description: API and schema contracts for remote-authority-reset.
---

# remote-authority-reset Contracts

## Core Contract Statement

Runtime authority contract is singular:
- Rust backend is the only memory/RAG authority.
- TypeScript adapter and context-engine are integration/orchestration layers only.
- No supported runtime mode may use local-authority persistence/retrieval execution.

## API Contracts

### Rust backend contracts (authoritative)
- generic recall;
- reflection recall;
- explicit write/update/delete/list/stats;
- reflection job enqueue/status;
- ACL/scope derivation and access filtering semantics.

### Thin adapter contracts
- convert OpenClaw runtime context into backend actor envelope;
- attach trusted principal headers and request IDs;
- generate idempotency keys where required;
- map backend errors into OpenClaw-facing responses;
- reject missing-principal writes/enqueue operations.

### Local context-engine contracts
- decide prompt-time recall/injection timing;
- render `<relevant-memories>`, `<inherited-rules>`, `<error-detected>` blocks;
- maintain session-local dedupe/suppression/error-signal state;
- consume backend-authoritative rows only.

## Runtime Config Contract (Post-Deletion Target)

Required runtime contract:
- remote backend config must be present and valid.
- local-authority config fields are not part of active runtime schema.

Contract implications:
- no active runtime `embedding` local execution config;
- no active runtime `dbPath`/`retrieval`/`scopes` local authority config;
- no `mdMirror` or `memoryReflection.storeToLanceDB` runtime behavior.

## Shared Type Ownership

- backend DTO ownership: `src/backend-client/types.ts`
- runtime identity resolution: `src/backend-client/runtime-context.ts`
- tool transport mapping: `src/backend-tools.ts`
- context-engine planner/render state: `src/context/*`

Constraint:
- permanent modules must not import deleted local-authority module types (`store.ts`, `retriever.ts`, `embedder.ts`).

## Event and Hook Contracts

Local hook ownership remains in plugin runtime:
- `before_agent_start`
- `before_prompt_build`
- `after_tool_call`
- `agent_end`
- `command:new`
- `command:reset`

Contract rule:
- backend does not register OpenClaw hooks;
- adapter/context-engine keep hook orchestration;
- backend remains data-plane authority only.

## Error Model Contract

- recall/injection paths: fail-open.
- write/update/delete/list/stats/enqueue paths: explicit error surfacing; principal enforcement retained.
- reflection enqueue: non-blocking for conversation flow, observable through logs/status.

## Compatibility and Deletion Contract

During deletion project:
- local-authority branches are treated as removal backlog, not supported mode.
- each deletion stage must pass explicit gate commands before next stage.
- rollback unit is stage commit, not partial hot edits.

Contract source of truth for staged execution:
- `remote-only-local-authority-removal-plan.md`
- `task-plans/phase-2-remote-authority-reset.md`
- `task-plans/phase-3-remote-authority-reset.md`
- `task-plans/phase-4-remote-authority-reset.md`
