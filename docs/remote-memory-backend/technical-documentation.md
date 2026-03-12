---
description: Canonical technical architecture for the remote Rust memory backend split.
---

# remote-memory-backend Technical Documentation

## Canonical architecture

Target runtime architecture:

1. **Remote memory backend**
   - Rust service
   - LanceDB for memory/reflection storage
   - SQLite job table for reflection enqueue/status
   - owns ACL, scope derivation, retrieval/ranking, model config, gateway config, reflection execution, persistence

2. **Local integration shell**
   - OpenClaw plugin wiring in `index.ts`
   - tool registration and lifecycle hook registration
   - HTTP client adapter, retry/backoff, auth headers, fail-open handling

3. **Local context orchestration**
   - `src/context/auto-recall-orchestrator.ts`
   - `src/context/reflection-prompt-planner.ts`
   - `src/context/session-exposure-state.ts`
   - `src/context/prompt-block-renderer.ts`
   - owns prompt gating, block rendering, repeated suppression, and error-signal dedupe

## Key constraints and non-goals

Constraints:

- remote backend is the only authority for ACL and scope;
- shell does not transmit requested scopes or provider config;
- shell does not implement local fallback backend behavior;
- backend config comes from static TOML only in MVP;
- reflection execution is backend-owned and async;
- `/new` and `/reset` must not block on reflection completion.

Non-goals:

- multi-node coordination;
- broker-backed distributed job execution;
- moving prompt rendering or session-local orchestration into the backend;
- environment variable configuration for the backend MVP.

## Interfaces between components

### Shell to backend

Transport:

- REST over HTTP(S)
- bearer token auth
- request id on all authenticated calls
- idempotency key on write/job-enqueue calls

Shell sends:

- actor identity:
  - `userId`
  - `agentId`
  - `sessionId`
  - `sessionKey`
- operation-specific payloads:
  - query
  - transcript items
  - memory ids
  - pagination/filter inputs

Shell never sends:

- ACL rules
- scope rules
- requested scopes
- embedding/rerank/reflection provider config
- gateway config

### Backend to shell

Backend returns:

- already-authoritative recall rows;
- explicit write/delete results;
- async reflection job status;
- stable structured errors with retry hints.

Backend response semantics:

- backend decides what rows are visible;
- backend decides target scope for writes;
- backend decides final ranking and selection before sending recall rows.

### Shell to local orchestration

The local shell adapter should expose contracts shaped for local orchestration, not raw REST responses:

- generic recall rows for auto-recall/manual-search;
- reflection recall rows for inherited-rules;
- explicit store/delete/list/stats methods for tool and CLI flows;
- reflection job enqueue/status methods for `/new` and `/reset`.

## Operational behavior

### Startup

- `index.ts` loads local shell config:
  - backend base URL
  - backend auth token
  - local OpenClaw integration flags only
- backend boots from TOML:
  - auth tokens
  - ACL/scope policy
  - embedding/rerank/reflection provider config
  - LanceDB path
  - SQLite job DB path

### Runtime modes

- generic recall:
  - shell decides whether recall should run;
  - backend decides what to return.

- reflection recall:
  - shell decides whether inherited-rules should be built this turn;
  - backend decides which reflection rows are visible and ranked.

- auto-capture:
  - shell submits transcript items;
  - backend decides extraction, dedupe, update/delete/noop behavior.

- `/new` and `/reset`:
  - shell enqueues reflection job;
  - backend performs reflection generation asynchronously;
  - later local recall reads persisted reflection rows normally.

## Observability and error handling

Backend should expose:

- health endpoint;
- structured error schema;
- reflection job status endpoint;
- stats endpoints for shell/admin traffic.

Shell should log:

- recall failures as warnings and continue;
- explicit tool write/delete failures as surfaced errors;
- reflection enqueue failures as warnings without blocking conversation.

Expected error behavior:

- recall path: fail-open;
- write/delete path: fail-closed to caller;
- reflection enqueue path: fail-open for conversation, but visible in logs;
- reflection worker failure: persisted in job status only.

## Security model and hardening notes

- ACL and scope remain backend-owned; shell must not reconstruct them locally.
- admin-token bypass applies only to explicitly marked management endpoints.
- error payloads must not leak secrets or raw upstream provider payloads.
- config TOML contains secrets in MVP and therefore must be file-permission hardened, for example `0600`.
- shell and backend tokens should be distinct from admin tokens.

## Test strategy mapping

Documentation checks:

```bash
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/remote-memory-backend
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/remote-memory-backend README.md
```

Implementation mapping:

- backend schema/contract tests for all endpoints;
- backend integration tests for LanceDB and SQLite job flows;
- shell adapter tests for auth, retry, and error translation;
- local orchestration tests to confirm:
  - recall failure remains fail-open;
  - tool failures surface;
  - reflection jobs are non-blocking;
  - local session state remains under `src/context/*`.

## Rollback and migration guardrails

- rollback must preserve singular authority:
  - either the local legacy backend path is authoritative,
  - or the remote backend is authoritative,
  - never both at once.
- no mixed scope/ACL ownership is allowed during migration.
- if the remote migration regresses, revert shell wiring to the local backend path entirely rather than introducing partial fallback behavior.
