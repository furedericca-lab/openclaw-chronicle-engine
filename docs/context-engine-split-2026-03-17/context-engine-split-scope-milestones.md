description: 2026-03-17 scope and milestone snapshot for context-engine-split.
---

# context-engine-split Scope and Milestones

Snapshot note:
- this file records the refreshed milestone framing for the 2026-03-17 internal split design state;
- current runtime authority and current config cutover status are documented elsewhere.

## In Scope

- Refactor internal code so prompt/context orchestration is no longer owned directly by `index.ts`.
- Introduce explicit structured seams for:
  - generic auto-recall row orchestration,
  - reflection recall row orchestration,
  - error reminder exposure,
  - session-local recall suppression state,
  - prompt block rendering.
- Keep current plugin kind, config schema, tool names, and memory slot behavior unchanged.
- Update tests and docs to describe the new boundaries accurately.

## Out of Scope

- Changing `openclaw.plugin.json` from `memory` to `contextEngine`.
- Adding a new shipped ContextEngine plugin entrypoint.
- Changing backend-owned retrieval/ranking/scope authority.
- Reworking backend retrieval scoring/rerank behavior except where needed to preserve adapter parity.
- Adding DAG-based transcript compaction or lossless session persistence.

## Milestones

### Milestone 1 — Contract and seam map
Acceptance gate:
- Planning docs identify exact modules and hook paths to preserve.
- Target boundaries between backend and orchestration are explicit and reviewable.

### Milestone 2 — Internal orchestration extraction
Acceptance gate:
- New orchestration/provider modules exist.
- `index.ts` delegates prompt-time planning/rendering instead of owning most logic inline.
- No public config or tool contract change.

### Milestone 3 — Behavior-parity verification
Acceptance gate:
- Tests covering auto-recall, reflection recall, session-strategy/config behavior for that snapshot, and self-improvement paths pass.
- README/docs reflect internal architecture shift without claiming a completed external plugin-contract migration.

### Milestone 4 — ContextEngine handoff readiness
Acceptance gate:
- The repo contains a documented adapter plan showing how a future ContextEngine can consume the extracted seams.
- Residual references to old inline orchestration ownership are removed from docs and comments in touched areas.

## Dependencies

- Milestone 1 blocks all others.
- Milestone 2 depends on Milestone 1 docs being concrete enough to drive implementation.
- Milestone 3 depends on Milestone 2 code changes landing.
- Milestone 4 depends on Milestones 2-3 because it documents the validated seam set rather than speculative design.

## Exit Criteria

- `openclaw-chronicle-engine` still ships as a `memory` plugin and passes tests.
- Prompt orchestration logic is modular enough that a thin future ContextEngine adapter can be written without reopening retrieval/storage internals.
- Active-path verification evidence is recorded under `docs/archive/context-engine-split/task-plans/4phases-checklist.md`.
