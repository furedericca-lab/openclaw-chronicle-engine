---
description: Implementation research notes for the remote Rust memory backend migration.
---

# remote-memory-backend Implementation Research Notes

## Problem statement and current baseline

Current local backend-heavy modules in this repo:

- `src/store.ts` — LanceDB table initialization, CRUD, FTS, vector/BM25 primitives.
- `src/embedder.ts` — OpenAI-compatible embedding abstraction, error normalization, chunking, multi-key behavior.
- `src/retriever.ts` — hybrid retrieval, rerank, recency/decay/length weighting, final selection pre-processing.
- `src/scopes.ts` — scope naming and access logic.
- `src/reflection-store.ts` plus related reflection modules — reflection row/event persistence and recall helpers.
- `src/tools.ts` — memory tool semantics and self-improvement tool registration.

Current local orchestration modules already separated from storage concerns:

- `src/context/auto-recall-orchestrator.ts`
- `src/context/reflection-prompt-planner.ts`
- `src/context/session-exposure-state.ts`
- `src/context/prompt-block-renderer.ts`

Current runtime entrypoint:

- `index.ts` still constructs backend objects locally and wires them into OpenClaw hooks/tools.

## Gap analysis with evidence

1. **Backend authority is still local.**
   Evidence: `index.ts` constructs `MemoryStore`, embedder, retriever, scope manager, and reflection persistence dependencies directly.

2. **Current orchestration expects local backend dependency contracts that will need HTTP-backed replacements.**
   Evidence:
   - `createAutoRecallPlanner(...).plan(...)` currently expects local retrieval/scoping helpers rather than a pure actor+query contract.
   - `createReflectionPromptPlanner(...)` currently expects local reflection access decisions rather than a pure actor+query+mode contract.

3. **Local scope resolution is incompatible with the selected authority model.**
   Evidence:
   - agreed redesign explicitly moves ACL and scope derivation into the backend;
   - current docs and code still describe local scope participation that must be removed from runtime contracts.

4. **Reflection execution is currently coupled to OpenClaw/plugin-local flows.**
   Evidence:
   - current reflection flow is triggered from local lifecycle hooks and consumes local runtime facilities;
   - agreed redesign moves execution and persistence to backend-owned async jobs while keeping only the trigger local.

5. **Config authority must move to the backend.**
   Evidence:
   - agreed redesign says the shell must not push provider or gateway config to the backend;
   - current codebase assumes embedder/rerank configuration is parsed inside the plugin.

6. **Current tool surface has explicit write/update semantics that the remote contract must preserve.**
   Evidence:
   - `src/tools.ts` exposes `memory_store` with explicit `text`, `importance`, and `category` inputs;
   - `src/tools.ts` also exposes `memory_update`, which means MVP parity is cleaner if the remote contract freezes a dedicated update path instead of removing the tool capability.

7. **Current local CLI surface is broader than the required remote MVP surface.**
   Evidence:
   - `cli.ts` includes `delete-bulk`, `export`, `import`, `reembed`, migration utilities, and FTS-focused paths;
   - those commands should not implicitly expand the remote MVP unless explicitly adopted into the frozen runtime contract.

## Architecture / implementation options and trade-offs

### Option 1: Replace local backend modules with direct REST calls inside `index.ts`

Pros:
- smallest immediate code delta.

Cons:
- keeps transport knowledge spread across entrypoint/hook handlers;
- local shell remains too thick;
- hard to test and migrate incrementally.

Decision:
- reject.

### Option 2: Introduce a local thin adapter that implements backend-facing contracts over REST

Pros:
- keeps transport concerns isolated;
- allows `src/context/*` to stay local with minimal signature changes;
- matches the selected thin-shell design.

Cons:
- requires careful contract definition to avoid leaking old local semantics.

Decision:
- selected.

### Option 3: Rewrite `src/context/*` to speak raw HTTP directly

Pros:
- fewer local layers.

Cons:
- transport leaks into orchestration code;
- makes prompt logic harder to test;
- contradicts the "thin adapter, local orchestration" design.

Decision:
- reject.

## Selected design and rationale

Selected design:

- remote Rust service becomes the memory authority;
- local shell keeps a REST adapter layer and local orchestration;
- ACL and scope derivation leave the shell completely;
- backend returns already-authoritative recall rows, not intermediate retrieval results requiring local policy decisions;
- admin capabilities remain a separate control plane and are not part of the ordinary shell/context/tool contract;
- the initial frozen admin surface is limited to read-only health and job inspection endpoints, with optional read-only global stats;
- `/v1` runtime contracts follow additive-only backward compatibility; breaking changes require a new major version.

Frozen contract decisions added by this research pass:

- `POST /v1/memories/store` supports two distinct request shapes via `mode`: `tool-store` and `auto-capture`;
- `tool-store` preserves explicit `category` and `importance`, but never accepts scope from the shell;
- `POST /v1/memories/update` is retained as a dedicated endpoint for MVP parity with the current tool surface;
- `POST /v1/memories/stats` replaces a GET/query-shaped data-plane stats route to stay consistent with the actor-envelope rule;
- reflection job status remains caller-scoped on the data plane and operator-global only on admin routes;
- stable recall DTOs do not expose raw vector/BM25/rerank scoring breakdowns.

Recommended local target shape:

- `src/backend-client/*` or similarly named local adapter modules:
  - HTTP client and auth headers
  - retry/backoff
  - DTO translation between REST payloads and local orchestration/tool needs on the data plane only
- `src/context/*` updated to consume backend-returned authoritative rows directly and stop calling local scope authority helpers
- `index.ts` updated to wire backend client dependencies instead of local storage/retrieval primitives

## Test and validation strategy

Documentation validation:

```bash
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/remote-memory-backend
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/remote-memory-backend README.md
```

Implementation-time validation expected from this research:

- backend contract tests per endpoint and status model;
- shell-side unit tests for adapter error translation;
- local orchestration tests proving:
  - recall failures remain fail-open;
  - tool write/update/delete failures surface to callers;
  - `/new` and `/reset` do not block on reflection completion;
  - local session-local state remains in `src/context/*`, not the adapter.

Contract points that should receive explicit verification:

- `tool-store` vs `auto-capture` body validation;
- category enum validation and defaulting behavior;
- stats actor-envelope handling on a POST route;
- reflection job visibility for same-principal user token vs admin token;
- list ordering and `nextOffset=null` behavior on the final page.

## Risks, assumptions, remaining open questions

Risks:

- accidental mixed authority if local shell still computes effective scopes while backend also enforces ACL;
- overexposing backend scoring internals in DTOs and creating unnecessary compatibility burden;
- leaking control-plane/admin semantics into ordinary runtime contracts.

Assumptions:

- Rust integration with LanceDB is feasible for the selected backend;
- SQLite is sufficient for single-instance reflection job tracking in MVP;
- static TOML config is acceptable for initial deployment stability.

Remaining open questions:

- Whether shell-side list/stats tooling needs richer pagination/filter DTOs after the frozen MVP, beyond the current minimum.
