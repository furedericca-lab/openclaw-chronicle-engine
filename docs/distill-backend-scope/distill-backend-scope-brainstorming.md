---
description: Brainstorming and decision framing for distill-backend-scope.
---

# distill-backend-scope Brainstorming

## Problem

The repo still contains a sidecar-style distillation path:

- `scripts/jsonl_distill.py`
- `examples/new-session-distill/*`

That path is not part of the current Rust remote backend authority model, but it still encodes useful transcript-ingestion and reduction techniques.

The decision problem is not whether to preserve the old sidecar as-is. The decision problem is:

- which distill-related capabilities should become backend-native;
- which current sidecar parts should be demoted, archived, or deleted as technical debt cleanup;
- how to express this clearly in the remote backend architecture docs.

## Scope

This scope plans backend-native distill capability only at the architecture/contract level.

It covers:

- transcript-tail ingestion and cursoring concepts from `scripts/jsonl_distill.py`;
- noise filtering / transcript cleaning rules from `scripts/jsonl_distill.py`;
- lesson extraction / dedupe / reduce concepts from `examples/new-session-distill/worker/lesson-extract-worker.mjs`;
- how a future distill job surface should relate to existing reflection jobs and auto-capture;
- cleanup/disposition strategy for current distill residue under `scripts/`, `examples/`, and tests;
- documentation corrections for `docs/remote-memory-backend-2026-03-17`.

It does not implement Rust backend distill jobs in this batch.

## Constraints

- remote backend remains the only authority for persistence, ACL, scope, and async job ownership;
- shell must not regrow a sidecar authority path for session distillation;
- `/new` and `/reset` must remain non-blocking;
- the existing reflection and auto-capture contracts must remain understandable as separate capabilities;
- example code may remain as historical reference, but it must not read like the canonical path.

## Options

### Option 1: Leave distill as sidecar/example only

Pros:

- zero backend complexity;
- no new API surface.

Cons:

- keeps useful ingestion logic outside the backend;
- leaves architectural drift unexplained;
- encourages future re-use of sidecar import paths.

### Option 2: Rebuild a backend-native distill job surface

Pros:

- aligns transcript distillation with the remote authority model;
- allows caller-scoped ownership, idempotency, observability, and persistence;
- reuses backend async job patterns already established by reflection jobs.

Cons:

- introduces a new job family and provider-execution surface;
- requires careful separation from reflection and auto-capture semantics.

### Option 2b: Rebuild backend-native distill and explicitly clean residue in phases

Pros:

- gives the repo a clear cleanup path instead of indefinite sidecar drift;
- makes it obvious which files are temporary reference vs future deletion targets.

Cons:

- requires more explicit milestone and archival discipline;
- may remove convenient ad hoc utilities before backend parity if sequenced badly.

### Option 3: Fold distill into reflection

Pros:

- fewer public concepts;
- no separate distill job endpoint.

Cons:

- reflection and distill have different inputs and outputs;
- makes the reflection scope too broad;
- weakens operator and developer understanding of why transcript distillation exists.

## Decision

Choose **Option 2b**.

Planned architecture position:

- keep `reflection` as backend-owned async reflective memory generation for `/new` and `/reset`;
- keep `auto-capture` as backend-owned transcript-to-memory mutation on ordinary conversation/tool flows;
- define `distill` as a future backend-native async transcript distillation surface for higher-cost, transcript-wide lesson extraction or governance-oriented outputs;
- freeze the current sidecar residue into explicit cleanup buckets:
  - `keep as temporary migration reference`
  - `demote to example-only`
  - `archive/delete after backend-native distill parity`

## Risks

- future backend-native distill work could accidentally duplicate reflection semantics;
- transcript ingestion can become an implicit second source-of-truth if cursor/state ownership is not clearly defined;
- if cleanup disposition is not explicit, maintainers may still treat `jsonl_distill.py` as a semi-supported production path.

## Open Questions

- should transcript sourcing for distill read session JSONL directly, or should the shell provide transcript items to backend jobs explicitly;
- should distill persist ordinary memory rows, structured lesson/governance artifacts, or both;
- should the first backend-native distill mode be `session-lessons`, `governance-candidates`, or a narrower operator digest mode;
- should distill jobs share the existing reflection job table or get a separate async job family.
