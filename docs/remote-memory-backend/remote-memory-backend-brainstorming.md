---
description: Architecture brainstorming for the remote Rust memory backend split.
---

# remote-memory-backend Brainstorming

## Problem

`memory-lancedb-pro` currently keeps storage, retrieval, scope logic, reflection persistence, and OpenClaw integration in one local TypeScript runtime. The planned redesign wants:

- a remote backend that is the only memory authority;
- a thin local shell that keeps OpenClaw lifecycle and tool integration;
- local `src/context/*` orchestration that still decides prompt-time injection timing and rendering.

The main design risk is not transport. The real risk is authority drift: if local shell and remote backend both own scope, ACL, or provider configuration, the system becomes nondeterministic and hard to debug.

## Design options

### Option 1: Remote storage only

Shape:
- move LanceDB persistence to a remote service;
- keep retrieval ranking, scope resolution, ACL, and reflection logic locally.

Pros:
- minimal backend scope;
- fastest short-term implementation.

Cons:
- wrong split for long-term maintenance;
- local shell still owns too much domain logic;
- impossible to make remote backend the true source of truth;
- future multi-client usage would still depend on OpenClaw-local policy.

Decision:
- reject.

### Option 2: Remote backend as full memory authority with thin local shell

Shape:
- remote backend owns storage, retrieval, scoring, rerank, scope/ACL, reflection execution, and persistence;
- local shell owns OpenClaw hook binding, tool binding, HTTP retry, fail-open behavior, and `src/context/*`.

Pros:
- clean authority model;
- backend can be reused outside OpenClaw later;
- local shell stays small and reviewable;
- prompt orchestration remains local where session-local suppression/dedupe already lives.

Cons:
- larger initial migration than remote-storage-only;
- requires a real API contract and migration plan.

Decision:
- selected.

### Option 3: Move both backend and prompt orchestration remote

Shape:
- backend performs retrieval and prompt-block planning;
- local shell becomes little more than a proxy.

Pros:
- minimal local behavior.

Cons:
- pushes OpenClaw-specific prompt semantics into the backend;
- makes session-local state harder to reason about;
- over-couples backend to current prompt tags and OpenClaw event surfaces.

Decision:
- reject for MVP.

## Key decisions frozen for this scope

- Backend runtime: `Rust`.
- Backend storage: `LanceDB`.
- Reflection job queue/status: `SQLite job table`.
- Backend config source: static `TOML` only for MVP.
- Backend is the only authority for:
  - ACL
  - scope derivation
  - embedding/rerank/reflection model config
  - gateway config
  - retrieval ranking and final selection
  - reflection execution and persistence
- Local shell does not send requested scopes or provider config.
- Local shell does not implement fallback backend behavior.
- `/new` and `/reset` trigger async reflection jobs locally but do not wait for job completion.

## Stable architecture direction

Recommended split:

1. Remote backend
- REST API for recall/store/delete/list/stats/reflection job endpoints.
- LanceDB-backed memory and reflection data.
- SQLite-backed reflection job queue/status table.
- ACL and scope enforcement before any query/write path.
- OpenAI-compatible upstream clients configured from backend TOML.

2. Local shell
- replace local `store/embedder/retriever/scopes/reflection-store/tools` dependencies with an HTTP client adapter;
- preserve `src/context/*` and session-local state;
- preserve fail-open recall behavior and explicit tool error surfacing.

3. Local orchestration
- `src/context/*` continues to decide:
  - whether to inject;
  - how to render `<relevant-memories>`, `<inherited-rules>`, `<error-detected>`;
  - repeated-suppression and reflection error-signal dedupe.

## Open design questions to resolve in implementation docs

- Whether admin-only endpoints should be implemented in MVP phase 1 or reserved and delivered in a later phase.
- Whether shell-side list/stats tooling needs richer pagination/filter DTOs in phase 1 than the current minimum.
- Whether reflection job result payload should expose timestamps and lightweight diagnostics in addition to aggregate counts.
