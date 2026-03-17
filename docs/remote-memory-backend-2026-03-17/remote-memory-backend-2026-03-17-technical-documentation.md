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
   - additionally owns backend-native distill job family with enqueue/status plus the initial `inline-messages` executor slice in this snapshot refresh

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
- shell transmits actor identity plus operation payloads only;
- shell does not transmit requested scopes, scope overrides, or provider config;
- shell does not implement local fallback backend behavior;
- backend config comes from static TOML only in MVP;
- reflection execution is backend-owned and async;
- `/new` and `/reset` must not block on reflection completion.
- transcript distill is fully shipped in this snapshot for both `inline-messages` and backend-owned `session-transcript` execution.

Snapshot-boundary note:

- this document freezes the backend-authority architecture and backend `/v1` transport surface as of the 2026-03-17 snapshot;
- it does not freeze later plugin/package release-line semantics, plugin config compatibility policy, or the exact location of test-only helper files.

Non-goals:

- multi-node coordination;
- broker-backed distributed job execution;
- moving prompt rendering or session-local orchestration into the backend;
- preserving example sidecar distiller flows as canonical runtime architecture;
- environment variable configuration for the backend MVP.

## Interfaces between components

### Shell to backend

Transport:

- REST over HTTP(S)
- bearer token auth
- request id on all authenticated calls
- trusted principal headers on all data-plane calls:
  - `X-Auth-User-Id`
  - `X-Auth-Agent-Id`
- idempotency key on write/job-enqueue calls

Shell sends:

- actor identity:
  - `userId`
  - `agentId`
  - `sessionKey`
  - `sessionId`
- operation-specific payloads:
  - query
  - transcript items
  - memory ids
  - pagination/filter inputs allowed by the frozen contract

Actor semantics:

- `sessionKey` is the stable session/conversation identity used for logical provenance and audit correlation;
- `sessionId` is an ephemeral runtime-instance diagnostic field;
- shell must not assume `sessionId` stability across retries, restarts, or handoffs;
- shell/runtime must not derive principal ownership (`userId`, `agentId`) from `sessionKey`;
- backend data-plane ownership and authorization must not require stable `sessionId`.
- backend authorizes principal identity from trusted runtime headers and requires actor `userId`/`agentId` to match those headers.

Shell never sends:

- ACL rules
- scope rules
- requested scopes
- embedding/rerank/reflection provider config
- gateway config

### Backend to shell

Backend returns:

- already-authoritative recall rows;
- explicit write/update/delete results;
- caller-scoped async reflection job status;
- stable structured errors with retry hints.

Backend response semantics:

- backend decides what rows are visible;
- backend decides target scope for writes;
- backend decides final ranking and selection before sending recall rows;
- ordinary runtime DTOs should expose only stable orchestration-facing semantics, not raw backend scoring internals.

### Shell to local orchestration

The local shell adapter should expose contracts shaped for local orchestration, not raw REST responses:

- generic recall rows keyed only by actor/query semantics;
- reflection recall rows keyed by stable high-level mode semantics;
- explicit store/update/delete/list/stats methods for tool and CLI-adjacent runtime flows on the data plane;
- reflection job enqueue/status methods for `/new` and `/reset`.

The local adapter should not expose admin/control-plane operations to local orchestration.

Capability boundary note:

- `reflection` is a shipped backend-owned async capability for reflective row generation;
- `auto-capture` is a shipped backend-owned mutation path for ordinary memory extraction;
- `distill` now ships backend-owned enqueue/status, transcript persistence, `inline-messages` execution, and backend-native `session-transcript` execution.

## Operational behavior

### Startup

- `index.ts` loads local shell config:
  - backend base URL
  - backend auth token
  - local OpenClaw integration flags only
- startup authority split:
  - local-authority mode initializes local LanceDB path validation, `MemoryStore`, embedder, retriever, access tracker, migrator, and local `memory-pro` CLI;
  - remote-authority mode initializes only remote transport wiring plus local orchestration modules under `src/context/*` and does not require local embedding config to boot.
- config validation boundary:
  - local-authority mode must fail fast at config-parse time when `embedding` is missing;
  - remote-authority mode accepts missing `embedding` when `remoteBackend.enabled=true` with valid `remoteBackend.baseURL` and `remoteBackend.authToken`.
- backend boots from TOML:
  - auth tokens
  - ACL/scope policy
  - embedding/rerank/reflection provider config
  - LanceDB path
  - SQLite job DB path

Later-scope clarification:

- later active scopes may intentionally remove plugin-layer legacy config aliases or reset plugin/package versioning without changing the backend startup or backend `/v1` transport model documented here.

### Runtime modes

- generic recall:
  - shell decides whether recall should run;
  - backend decides what to return.

- reflection recall:
  - shell decides whether inherited-rules should be built this turn;
  - backend decides which reflection rows are visible and ranked.

- explicit tool-store:
  - shell submits a single explicit memory payload with optional `category` and `importance`;
  - remote tool contract does not expose `scope` input;
  - shell never submits target scope;
  - backend validates the frozen category enum, applies default importance when omitted, and chooses the authoritative target scope.

- auto-capture:
  - shell submits transcript items;
  - backend decides extraction, dedupe, classification, update/delete/noop behavior.

- transcript distill:
  - canonical routes now exist:
    - `POST /v1/distill/jobs`
    - `GET /v1/distill/jobs/{jobId}`
    - `POST /v1/session-transcripts/append`
  - current implementation supports transcript append, `inline-messages` execution, backend-owned `session-transcript` execution, artifact persistence, and optional memory-row persistence;
  - reducer shaping is deterministic and backend-owned, including evidence gating, duplicate suppression, and stable artifact ranking;
  - sidecar queue-file and local-import residue is no longer part of the active runtime architecture.

- explicit memory update:
  - shell submits `memoryId` plus a constrained patch payload;
  - backend enforces ACL, keeps scope authority, and performs an in-place row update (no delete-then-insert loss window).

- list/stats:
  - shell requests caller-visible data-plane views only;
  - backend owns ordering, pagination semantics, and access filtering.

- `/new` and `/reset`:
  - shell normalizes trigger contract to `new` or `reset` before enqueue;
  - shell enqueues reflection job;
  - backend performs reflection generation asynchronously;
  - later local recall reads persisted reflection rows normally;
  - local prompt-session reflection state is cleared after hook completion.

## Observability and error handling

Backend should expose:

- health endpoint;
- structured error schema;
- reflection job status endpoint;
- data-plane stats endpoint for content counts;
- separate admin/control-plane endpoints for operator-only health and job inspection.

Initial admin/control-plane contract surface (reserved in `/v1`):

- `GET /v1/admin/health`
- `GET /v1/admin/jobs`
- `GET /v1/admin/jobs/{jobId}`
- optional read-only extension: `GET /v1/admin/stats`

Deferred from the initial frozen admin surface:

- config writes
- policy writes
- memory mutation endpoints
- bulk operator endpoints

Current implementation status (2026-03-17 refresh):

- admin/control-plane routes are not yet exposed in this backend build;
- admin tokens are therefore not accepted on any runtime path;
- data-plane middleware continues to require runtime bearer token + runtime principal headers only;
- this keeps admin-token bypass isolated to explicit future control-plane routes, with no active bypass path in current MVP.
- backend now additionally exposes debug-scoped retrieval trace routes:
  - `POST /v1/debug/recall/generic`
  - `POST /v1/debug/recall/reflection`
- these debug routes are principal-scoped, return structured trace payloads outside ordinary `/v1/recall/*` DTO rows, and do not imply that the reserved `/v1/admin/*` control-plane surface has shipped.

Data-plane route notes:

- `POST /v1/memories/stats` is the canonical caller-scoped stats endpoint in MVP;
- `GET /v1/reflection/jobs/{jobId}` remains a caller-scoped diagnostic route, not an operator-global inspection route.
- distill enqueue/status routes are now shipped as caller-scoped runtime data-plane triggers/diagnostics, and `inline-messages` requests execute asynchronously to terminal status with persisted artifacts.

Shell should log:

- recall failures as warnings and continue;
- recall skips caused by missing runtime principal identity (`userId`/`agentId`) as warnings and continue;
- explicit tool write/update/delete failures as surfaced errors;
- write/list/stats/job-enqueue blocks caused by missing runtime principal identity as explicit errors/warnings;
- reflection enqueue failures as warnings without blocking conversation.

Expected error behavior:

- recall path: fail-open;
- write/update/delete path: fail-closed to caller;
- list/stats/job-enqueue path: fail-closed when trusted runtime principal identity is unavailable;
- reflection enqueue path: fail-open for conversation, but visible in logs;
- reflection worker failure: persisted in job status only.

Idempotency lifecycle behavior in current MVP:

- write/delete/update/job-enqueue calls reserve idempotency keys in SQLite and transition through `reserved -> in_progress -> completed|failed`;
- if the protected side effect fails before completion, the key is recorded as `failed` and may be retried safely with the same key only when payload fingerprint matches;
- duplicate requests for already completed keys currently return `409 IDEMPOTENCY_CONFLICT` because full response replay is not yet implemented.

## Security model and hardening notes

- ACL and scope remain backend-owned; shell must not reconstruct them locally.
- trusted runtime identity headers (`X-Auth-User-Id`, `X-Auth-Agent-Id`) define the caller principal boundary for the data plane and must be injected by the auth gateway layer, not blindly trusted as client self-assertions.
- shell must not synthesize data-plane principal ownership (`userId`, `agentId`) from static fallback config; missing runtime principal identity must be handled via skip/block behavior, not fabrication.
- `sessionKey` is provenance/correlation only and must never act as a shadow principal source.
- admin endpoints belong to a separate control plane, not the ordinary actor/data-plane contract.
- admin-token bypass applies only to explicitly marked management endpoints and must remain auditable.
- until admin routes are explicitly implemented, admin tokens must not grant access to any data-plane endpoint.
- debug-scoped retrieval trace routes are not admin-token routes; they remain on the runtime principal boundary and exist to expose inspectable recall traces without widening ordinary recall DTOs.
- backend-owned transcript persistence must remain the only supported source for `session-transcript` distill execution.
- all admin requests must emit an audit record with request id, operator identity, endpoint/method, target selector, timestamp, result status, and status code;
- admin mutations must additionally require and record a reason field.
- error payloads must not leak secrets or raw upstream provider payloads.
- config TOML contains secrets in MVP and therefore must be file-permission hardened, for example `0600`.
- shell and backend tokens should be distinct from admin tokens.

Reflection job ownership and visibility rules:

- enqueue records the owner principal from actor identity;
- user-token data-plane status is visible only to the same `(userId, agentId)` principal;
- `sessionKey` is recorded for provenance and audit enrichment;
- `sessionId` must not be the deciding factor for long-lived visibility;
- operator-global job inspection belongs only to admin endpoints.

## API versioning policy

- `/v1` permits backward-compatible additive changes only.
- Existing request/response fields and semantics must remain stable within `/v1`.
- Breaking schema or semantic changes require a new major API version.
- this rule applies to backend HTTP contracts only.
- plugin/package config-schema or release-version resets outside backend DTOs are out of scope for this policy and may be handled by later active scopes.

## MVP runtime parity boundary

Remote MVP parity is intentionally narrower than today's local CLI surface.

Required remote MVP parity:

- `POST /v1/recall/generic`
- `POST /v1/recall/reflection`
- `POST /v1/memories/store`
- `POST /v1/memories/update`
- `POST /v1/memories/delete`
- `POST /v1/memories/list`
- `POST /v1/memories/stats`
- `POST /v1/reflection/jobs`
- `GET /v1/reflection/jobs/{jobId}`

Explicitly deferred from remote MVP parity:

- `delete-bulk`
- `export`
- `import`
- `reembed`
- transcript distill endpoints
- migration utilities
- FTS maintenance and deep retrieval telemetry endpoints
- admin mutation/config endpoints

Contract rule:

- existing local CLI/operator commands may continue to exist during migration, but they do not automatically imply required remote API parity in MVP.
- control-plane admin route implementation remains deferred in current MVP and must not be inferred from local operator command availability.

## Test strategy mapping

Documentation checks:

```bash
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/remote-memory-backend
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/remote-memory-backend README.md
```

Implementation mapping:

- backend schema/contract tests for all frozen MVP endpoints;
- backend integration tests for LanceDB and SQLite job flows;
- shell adapter tests for auth, retry, and error translation;
- local orchestration tests to confirm:
  - recall failure remains fail-open;
  - tool write/update/delete failures surface;
  - reflection jobs are non-blocking;
  - local session state remains under `src/context/*`.

Recommended contract-coverage focus added by this freeze:

- `tool-store` vs `auto-capture` request-shape validation;
- frozen category enum rejection behavior;
- `POST /v1/memories/stats` actor-envelope behavior;
- `GET /v1/reflection/jobs/{jobId}` ownership and token-boundary behavior;
- list ordering and `nextOffset=null` last-page semantics.

Phase 4 verification batch (2026-03-13) evidence anchors:

- `test/remote-backend-shell-integration.test.mjs`
  - fail-open generic recall when backend returns failure;
  - fail-open reflection recall when backend returns failure;
  - surfaced write/update/delete failures from remote backend;
  - non-blocking reflection enqueue with explicit failure observability.
- `backend/tests/phase2_contract_semantics.rs`
  - caller-scoped reflection job visibility;
  - stats actor-envelope principal enforcement;
  - list ordering and terminal pagination semantics;
  - forbidden scope and frozen-category rejection;
  - admin token cannot bypass data-plane routes and unauthorized payloads do not leak token values.

## Rollback and migration guardrails

- rollback must preserve singular authority:
  - either the local legacy backend path is authoritative,
  - or the remote backend is authoritative,
  - never both at once.
- no mixed scope/ACL ownership is allowed during migration.
- if the remote migration regresses, revert shell wiring to the local backend path entirely rather than introducing partial fallback behavior.
- migration acceptance should explicitly confirm that runtime DTOs did not regain scoring-internal coupling or shell-side scope authority.

Cutover runbook (local shell -> remote backend authority):

1. Verify backend contract health before cutover:
   - backend: `CARGO_BUILD_JOBS=1 ~/.cargo/bin/cargo test --locked --test phase2_contract_semantics -- --nocapture`
   - shell: `node --test --test-name-pattern='.' test/remote-backend-shell-integration.test.mjs`
2. Enable remote mode as a single switch in plugin config:
   - set `remoteBackend.enabled=true` with valid `baseURL` and `authToken`;
   - keep local `src/context/*` orchestration enabled, but keep storage/ACL authority remote-only.
3. Verify runtime behavior immediately after cutover:
   - recall continues fail-open under backend recall failures;
   - write/update/delete failures are surfaced to callers;
   - `/new` and `/reset` enqueue reflection jobs asynchronously without blocking interaction.
4. Confirm parity boundary:
   - only frozen MVP runtime endpoints are treated as release-critical;
   - deferred operator/admin surfaces remain explicitly deferred.

Rollback runbook (remote -> local authority):

1. Disable remote mode in one step (`remoteBackend.enabled=false`) and restore local authoritative wiring.
2. Do not retain any partial remote write/list/stats/recall path after rollback.
3. Re-run baseline verification:
   - `npm test`
4. Confirm no mixed-authority residue:
   - no shell payloads should include local scope authority fields in remote mode;
   - no remote auth headers should be required once local mode is restored.
