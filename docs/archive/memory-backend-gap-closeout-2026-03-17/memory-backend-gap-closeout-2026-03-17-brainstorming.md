---
description: Brainstorming and decision framing for memory-backend-gap-closeout-2026-03-17.
---

# memory-backend-gap-closeout-2026-03-17 Brainstorming

## Problem

The current runtime story says memory authority is backend-owned, but the audit found three remaining practical gaps:

- `/new` and `/reset` reflection input still depends on plugin-local session file recovery and parsing;
- reflection job status exists in the backend and client, but not in the shipped plugin surface;
- legacy config and top-level TS residue still project a partly-local mental model.

## Scope

This scope closes the remaining backend-gap implementation work that is still actionable inside this repo:

- move reflection enqueue source resolution toward backend-owned transcript authority;
- expose caller-scoped reflection job status through the adapter/tool surface;
- tighten legacy config and residue handling so supported runtime behavior is less misleading.

## Constraints

- backend remains the only authority for persistence, recall, ranking, ACL, and reflection execution;
- prompt-local seams that only shape prompt injection are not in scope for migration;
- no local fallback runtime may be reintroduced;
- any status/debug/management surface must stay principal-scoped and fail closed when identity is missing.

## Options

### Option A: Minimal adapter closeout only

- add only `memory_reflection_status` tool;
- leave local session-file reflection recovery in place;
- keep legacy config and test-only residue as documented debt.

Trade-off:

- cheapest implementation;
- does not actually close the largest remaining authority gap.

### Option B: Full plugin-side cleanup without backend contract change

- remove local session-file recovery entirely;
- require reflection enqueue to depend only on current event messages or already-appended transcript rows;
- add reflection status tool and tighten legacy config handling.

Trade-off:

- cleanest plugin surface;
- risky if current runtime hooks do not yet guarantee adequate transcript availability before `/new` and `/reset`.

### Option C: Backend-gap closeout with bounded contract extension

- add a backend/client path for caller-scoped transcript-backed reflection source loading or reuse an equivalent backend-owned transcript query path;
- switch `/new` and `/reset` reflection enqueue away from local file parsing;
- add reflection status tool;
- perform bounded cleanup on legacy config and test-only residue.

Trade-off:

- slightly broader than adapter-only work;
- actually aligns runtime behavior with the documented backend-owned source-of-truth model.

## Decision

Choose Option C.

Reason:

- the audit’s highest-value gap is not naming debt but the remaining plugin-local source recovery before reflection enqueue;
- adding reflection status alone would leave the most important authority inconsistency untouched;
- legacy config/residue cleanup should land only after the runtime source path is made explicit.

## Ownership

- `backend/src/*`: transcript-backed reflection-source support and any required caller-scoped status/lookup contracts.
- `src/backend-client/*`: typed transport methods and DTOs for the selected reflection-source/status routes.
- `index.ts`: reflection hook rewiring away from local session file parsing.
- `src/backend-tools.ts`: management-gated reflection status tool.
- `openclaw.plugin.json`, `README.md`, `README_CN.md`: compatibility and operator guidance updates.

## Parity / Migration Notes

- prompt-local `adaptive-retrieval` and `setwise-v2` stay intentionally local;
- `query-expander.ts` and `reflection-store.ts` are cleanup targets only if production import proof remains empty;
- config compatibility removals should be staged conservatively, with warning-only behavior before deletion.

## Risks

- runtime hook timing may reveal that transcript-backed reflection needs an additional backend route rather than a pure adapter refactor;
- removing local session-file recovery too aggressively could regress `/reset` behavior if transcript append ordering is not explicit;
- exposing reflection status creates a new management surface that must stay gated and principal-scoped.

## Open Questions

- should reflection source loading be a dedicated backend route or piggyback on an existing transcript-owned abstraction?
- should `memory_reflection_status` be management-gated only, or always available because it is caller-scoped and non-mutating?
- should legacy config fields remain warning-only in this scope, or should some become hard errors?
