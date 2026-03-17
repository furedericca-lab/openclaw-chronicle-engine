---
description: Scope boundaries and milestones for memory-backend-gap-closeout-2026-03-17.
---

# memory-backend-gap-closeout-2026-03-17 Scope and Milestones

## In Scope

- close the reflection source authority gap for `/new` and `/reset`;
- expose caller-scoped reflection job status through the plugin adapter/tool surface;
- reduce legacy config and residual TS debt that still contradicts the backend-owned runtime story;
- update docs/schema/tests so the shipped behavior matches the code.

## Out of Scope

- migrating intentional prompt-local recall gating or post-selection seams into the backend;
- widening ordinary recall DTOs with debug internals;
- adding operator-global reflection inspection or admin auth changes;
- redesigning distill or generic recall ownership.

## Milestones

### Milestone 1 - Contract freeze and source-path decision

Acceptance gate:

- the selected backend-owned reflection source path is explicit as `POST /v1/reflection/source`;
- adapter surface for reflection status is frozen;
- cleanup targets are classified as runtime, compatibility, or test-only residue.

### Milestone 2 - Reflection runtime closeout

Acceptance gate:

- `/new` and `/reset` no longer depend on local session file recovery in the supported runtime path;
- reflection status surface is implemented and test-backed;
- principal and fail-closed behavior are explicit and verified.

### Milestone 3 - Compatibility and residue closeout

Acceptance gate:

- legacy config/docs/schema language matches the actual runtime;
- unused local helper placement no longer misleads readers about active authority;
- final checklist records evidence, residual risks, and archive/handoff notes.

## Dependencies

- Milestone 1 blocks all later work.
- Milestone 2 depends on Milestone 1.
- Milestone 3 depends on Milestones 1-2.

## Exit Criteria

- no supported runtime reflection path relies on local session file recovery;
- reflection status is reachable from the shipped plugin surface;
- docs and schema no longer imply that legacy local-reflection knobs are active behavior;
- residual risks are explicit if any test-only helper remains in top-level `src/`.

## Archive / Handoff Note

- follows the deleted audit scope `memory-backend-gap-audit-2026-03-17`, whose findings are now absorbed into this phased scope;
- when implementation is finished, archive this scope under `docs/archive/`.
