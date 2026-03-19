---
description: Brainstorming and decision framing for governance-behavioral-closeout-2026-03-19.
---

# governance-behavioral-closeout-2026-03-19 Brainstorming

## Problem

- Active runtime code still exposed legacy helper surfaces after the earlier unification:
  - governance tools still registered `self_improvement_*` aliases;
  - `src/self-improvement-tools.ts`, `src/context/reflection-prompt-planner.ts`, and `src/context/reflection-error-signals.ts` remained as wrapper modules;
  - `index.ts` still accepted hidden reflection-era config aliases such as `autoRecallExcludeReflection`, `inheritance-*`, and `includeKinds=["invariant" | "derived"]`;
  - README files still documented the old governance aliases.
- Backend adapter internals also still used `reflection` naming in places where the long-term concept is now behavioral guidance rather than a separate reflection workflow.

## Scope

- Remove the public governance alias/tool/module surfaces.
- Delete no-longer-needed shim files and canonicalize imports/tests/docs.
- Move adapter and backend internal helper naming toward behavioral-guidance wording where that does not churn the frozen backend wire/storage contract.
- Archive the previous `docs/autorecall-governance-unification-2026-03-18/` scope once this closeout lands.

## Constraints

- `distill` remains the only trajectory-derived generation authority.
- `autoRecall` remains the only prompt-time recall/injection orchestration surface.
- `governance` remains the only governance workflow/tooling surface.
- Broad backend contract churn is out of scope:
  - keep `/v1/recall/reflection` and `/v1/debug/recall/reflection`;
  - keep persisted `category=reflection` / `reflection_kind` storage fields;
  - keep backend response field `reflectionCount`.
- No separate release note / migration note is required for this scope.

## Options

### Option 1

Keep the existing compatibility aliases and shims, but stop mentioning them in docs.

Trade-off:
- Lowest implementation cost.
- Fails the closeout goal because the codebase still describes two names for the same governance surface and keeps dead wrapper modules.

### Option 2

Rename the entire backend wire/storage contract away from `reflection`.

Trade-off:
- Most semantically pure outcome.
- Too risky for a closeout scope because it would churn HTTP routes, persisted row/category assumptions, test fixtures, and archived design references at once.

### Option 3

Canonicalize all plugin/runtime-facing governance and behavioral-guidance surfaces, delete the wrapper modules, and rename only the safe adapter/backend internal helpers while keeping the backend wire/storage contract stable.

Trade-off:
- Removes the confusing active surfaces.
- Leaves a documented backend compatibility boundary where `reflection` still exists in routes and persisted row semantics.

## Decision

Choose option 3.

- Governance is governance-only in active tool registration, docs, and modules.
- Behavioral guidance is the neutral internal concept for prompt-time recall/injection and retry/debug handling.
- Backend compatibility boundary stays frozen at the route/storage layer and is documented explicitly instead of hidden behind more compatibility wrappers.

## Risks

- Removing `.learnings` read-through compatibility means legacy backlog directories are no longer auto-imported.
- Historical top-level design snapshots outside this scope still contain older terms and rely on the docs index/runtime architecture to clarify that they are not the current source of truth.
- The backend still returns `reflection` route names and persisted categories; callers must rely on the canonical adapter/docs wording rather than inferring semantics from raw route strings.

## Open Questions

- None for this closeout. The remaining `reflection` route/storage naming is an intentional boundary, not an unresolved blocker.
