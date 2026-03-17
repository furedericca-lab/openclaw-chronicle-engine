---
description: Canonical technical architecture for memory-backend-gap-closeout-2026-03-17.
---

# memory-backend-gap-closeout-2026-03-17 Technical Documentation

## Canonical Architecture

1. `backend/src/*` remains the memory and transcript authority.
2. `src/backend-client/*`, `src/backend-tools.ts`, and `index.ts` remain the adapter/runtime integration layer.
3. `src/context/*` remains prompt-local orchestration only.

This scope narrows one remaining inconsistency: reflection enqueue source recovery must align with backend-owned transcript authority rather than local session-file discovery.

## Key Constraints and Non-Goals

- no local-authority fallback;
- no local filesystem dependence for supported reflection history recovery after this scope;
- no change to prompt-local seams that only trim or gate already-returned backend rows;
- no unauthenticated or operator-global reflection status surface.

## Module Boundaries and Data Flow

Target runtime flow after closeout:

```text
/new or /reset
  -> plugin normalizes trigger
    -> backend-owned transcript source resolution
      -> POST /v1/reflection/jobs
        -> backend enqueues async reflection work
          -> optional caller-scoped status poll via plugin tool
```

Boundary changes in this scope:

- remove plugin-local session directory scanning from the supported reflection path;
- add a caller-scoped backend route `POST /v1/reflection/source` that resolves transcript-backed reflection input messages;
- add a management-gated reflection status tool on top of existing caller-scoped backend status;
- keep transcript append at `agent_end`.

## Interfaces and Contracts

- reflection source path:
  - `POST /v1/reflection/source`;
  - backend-owned;
  - caller-scoped;
  - no client-provided scope override.
- reflection status path:
  - caller-scoped;
  - management-gated;
  - structured error normalization reused from existing backend client.

## Ownership Boundary

- backend:
  - transcript truth
  - reflection execution
  - reflection status
- adapter:
  - tool exposure
  - runtime identity wiring
  - compatibility warnings
- prompt-local:
  - inherited-rules injection over backend-returned reflection rows only

## Security and Reliability

- all reflection management/read routes require runtime principal identity;
- `POST /v1/reflection/source` and `memory_reflection_status` must reject cross-principal access and missing-principal calls;
- status tool must not leak another principal’s jobs;
- local session file layout changes must no longer be able to break supported reflection behavior;
- any new backend route must keep `sessionKey` as provenance, not principal authority.

## Observability and Error Handling

- reflection enqueue remains non-blocking;
- reflection status gives explicit lifecycle visibility for debugging and operator workflow;
- migration diagnostics should clearly distinguish:
  - active prompt-local controls;
  - warning-only compatibility fields;
  - test/reference-only helper files.

## Test Strategy

- backend:
  - contract tests for `POST /v1/reflection/source` and reflection status assumptions
- plugin:
  - shell integration tests for `memory_reflection_status`
  - reflection hook regression tests for `/new` and `/reset`
  - config migration tests for warning/deprecation behavior
- repo hygiene:
  - import/use-site scans
  - placeholder/refactor text scans on the scope docs
