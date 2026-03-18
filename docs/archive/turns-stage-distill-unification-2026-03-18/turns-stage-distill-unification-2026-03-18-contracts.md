---
description: API and schema contracts for turns-stage-distill-unification-2026-03-18.
---

# turns-stage-distill-unification-2026-03-18 Contracts

## Contract Summary

This scope removes command-coupled reflection generation (`/new` / `/reset` -> reflection source/job -> persisted reflection rows) and consolidates all trajectory-to-new-knowledge generation under backend-native distill.

The new semantic split is:
- **distill** = the only write path that derives new knowledge from session trajectories
- **reflection recall** = optional read/injection path over already-persisted reflection-typed memories only if such a read path remains useful after cleanup
- **self-improvement** = local governance/logging/skill-promotion utilities, not trajectory distillation

## API Contracts

### Removed runtime/plugin behaviors

The following plugin/runtime behaviors are removed without compatibility shims:
- `command:new` reflection enqueue hook
- `command:reset` reflection enqueue hook
- plugin-side calls to `POST /v1/reflection/source`
- plugin-side calls to `POST /v1/reflection/jobs`
- plugin-side management/status tooling that exists only for the removed reflection job flow

The following docs/tests/config descriptions must be updated to match removal:
- any README section that describes `/new` / `/reset` reflection generation
- any config/test wording that implies command-bound reflection generation remains supported

### Distill trigger contract

Distill remains backend-native and cadence-driven.

Primary trigger:
- runtime `agent_end`
- count user turns from appended transcript items
- when accumulated completed user turns cross `distill.everyTurns`, enqueue one backend-native distill job

No command-surface trigger is required for knowledge generation.

### Distill mode contract

Existing public modes today:
- `session-lessons`
- `governance-candidates`

Planned semantic evolution in this scope:
- keep `session-lessons` as the canonical cadence-driven turns-stage extraction mode
- expand its extraction logic so it subsumes the useful output class previously produced by reflection-generation and the desired non-chunked lesson extraction path
- optional follow-up: introduce a more explicit mode name only if implementation proves a hard schema split is needed; avoid renaming in the first implementation pass unless necessary

### Source contract

Canonical source for cadence-driven knowledge generation:
- `session-transcript`

Allowed auxiliary/manual source for explicit management runs:
- `inline-messages`

Explicitly removed as a first-class generation path from plugin runtime orchestration:
- `reflection/source` command-triggered loading path for `/new` / `/reset`

## Shared Types / Schemas

### Turns-stage lesson extraction semantic contract

Turns-stage extraction means:
- reduction window is expressed in ordered conversational turns/messages already persisted as transcript rows
- extraction logic may aggregate evidence across multiple turns
- extraction must not depend on historical token-chunk map/reduce behavior
- extraction may still use staged internal reducers, but externally the semantics are turn/message scoped, not token-chunk scoped

### Output artifact contract

Distill-generated artifacts may include:
- durable lessons
- decision summaries
- prevention/fix guidance
- governance candidates
- follow-up-focus / next-turn-guidance as distill-owned artifact subtypes for formerly derived/open-loop style outputs

Promotion rules:
- stable decision / durable practice promotion requires at least two distinct evidence messages and either:
  - repeated stable-decision / durable-practice phrasing across at least two messages; or
  - corroborating cause/fix/prevention context spanning at least two messages
- when that gate is not met, the artifact must fall back to ordinary `Lesson`
- derived/open-loop style outputs must not be persisted through a separate reflection/invariant pipeline
- follow-up-focus / next-turn-guidance are downgraded artifact subtypes, not a standalone generation mode

### Memory write contract

Only distill writes new trajectory-derived memory rows.

If reflection-category memories remain in the model after this scope, they must be written by distill-owned logic, not by a separate reflection job pipeline.

## Event and Streaming Contracts

### Runtime/plugin hooks kept
- `agent_end` transcript append
- `agent_end` cadence evaluation for automatic distill
- prompt-time recall/injection hooks only if they do not generate new persisted knowledge

### Runtime/plugin hooks removed
- `command:new` knowledge-generation hook
- `command:reset` knowledge-generation hook

## Error Model

### Distill failure behavior
- transcript append failure remains isolated and visible in logs
- distill enqueue failure remains non-fatal to the user turn
- cadence state must not double-enqueue on duplicate `agent_end` deliveries

### Removed reflection-job failure surface
After cleanup, there should be no runtime path that can fail due to:
- reflection source load transport errors for command hooks
- reflection enqueue transport errors for command hooks

## Validation and Compatibility Rules

1. No compatibility cleanup is required for the removed `/new` / `/reset` reflection generation flow.
2. No rollback compatibility is required for removed reflection-generation behavior, schemas, or docs.
3. Command-triggered reflection-job tests should be deleted or rewritten to assert absence of registration/calls.
4. Distill cadence tests must remain green and become the primary knowledge-generation verification surface.
5. Documentation must describe turns-stage lesson extraction rather than token-chunk map-stage extraction.
6. Public semantics must remain clear: knowledge generation is cadence-driven under distill, not command-driven under reflection.
