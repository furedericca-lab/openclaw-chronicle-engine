---
description: Implementation research notes for memory-backend-gap-closeout-2026-03-17.
---

# memory-backend-gap-closeout-2026-03-17 Implementation Research Notes

## Baseline (Current State)

- reflection enqueue now resolves transcript-backed source messages through `POST /v1/reflection/source` before enqueueing the backend job;
- backend client exposes `enqueueReflectionJob()`, `loadReflectionSource()`, and `getReflectionJobStatus()`;
- plugin tools now expose `memory_reflection_status` alongside the existing distill/debug management surfaces when management tools are enabled;
- config parser still accepts legacy local-reflection fields, but the supported runtime treats them as compatibility-only values that warn and do not affect backend reflection execution;
- `src/query-expander.ts` and `src/reflection-store.ts` remain top-level test/reference helpers and still have no supported-runtime import path.

## Gap Analysis

1. Closed authority gap:
   - reflection execution is backend-owned;
   - reflection historical source recovery is now backend-owned as well.
2. Closed adapter gap:
   - reflection status now exists in backend, client, and tool/runtime surface.
3. Remaining migration debt:
   - operator-facing compatibility vocabulary still exists, but it is now explicitly documented as compatibility-only.

## Candidate Designs and Trade-offs

### Backend transcript lookup route

Pros:

- cleanest authority model;
- removes local filesystem dependency from reflection flow.

Cons:

- may require backend route or request-shape expansion.

### Reuse already-appended transcript only

Pros:

- smaller surface expansion.

Cons:

- may not cover `/reset` timing if the relevant transcript slice is not yet appended or addressable.

### Keep local recovery and only document it

Pros:

- minimal implementation work.

Cons:

- does not close the highest-priority audit finding.

## Selected Design

- phase 1 freezes the exact reflection-source contract and verification gate;
- phase 2 implements `POST /v1/reflection/source` plus `memory_reflection_status`;
- phase 3 cleans residual config and test-only helper placement/docs after the runtime path is stable.

## Ownership Boundary Notes

- backend owns transcript truth and reflection job lifecycle;
- adapter owns transport, tool exposure, and compatibility messaging;
- prompt-local context planners remain local but must only consume backend-returned reflection rows.

## Parity / Migration Notes

- `query-expander.ts` and `reflection-store.ts` are no longer parity obligations;
- prompt-local `adaptive-retrieval` and `setwise-v2` remain intentional seams;
- legacy config should be minimized after implementation, not before, to avoid removing the only migration breadcrumbs too early.

## Residue / Debt Disposition Notes

- `src/session-recovery.ts` has been deleted because reflection no longer uses local session path discovery in the supported runtime;
- `src/query-expander.ts` and `src/reflection-store.ts` remain explicitly classified as test/reference helpers, and import-proof remains empty outside tests;
- docs and schema text now treat parser-accepted legacy fields as compatibility-only controls, not active runtime authority.

## Validation Plan

- import/use-site scans for local reflection/session helpers;
- backend contract tests for `POST /v1/reflection/source`;
- Node integration coverage for reflection status tool behavior and missing-principal failure mode;
- regression coverage for `/new` and `/reset` reflection enqueue behavior.

## Risks and Assumptions

- assumes backend changes in this repo are acceptable for closing the authority gap;
- assumes management-gating reflection status is consistent with existing distill/debug surfaces;
- assumes test-only helper relocation can be staged after runtime path changes without losing auditability.
