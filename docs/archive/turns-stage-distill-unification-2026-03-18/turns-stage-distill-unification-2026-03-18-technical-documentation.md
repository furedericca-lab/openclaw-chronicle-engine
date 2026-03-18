---
description: Technical design for turns-stage-distill-unification-2026-03-18.
---

# turns-stage-distill-unification-2026-03-18 Technical Documentation

## Objective

Unify trajectory-derived knowledge generation under backend-native distill, rename the intended future extraction model to **turns-stage lesson extraction**, and delete command-triggered reflection generation without compatibility cleanup.

## Target Architecture

### Runtime/plugin layer
Responsibilities after refactor:
- append ordered transcript rows on `agent_end`
- track completed user turns per session
- enqueue distill when `distill.everyTurns` cadence is crossed
- optionally perform read-only reflection recall/prompt injection if that remains configured
- never generate or persist new knowledge because `/new` or `/reset` was invoked

### Backend distill layer
Responsibilities after refactor:
- own all trajectory-derived extraction
- accept `session-transcript` and `inline-messages` sources
- perform turns-stage lesson extraction
- emit artifacts and optional memory rows
- absorb the useful retained output class that previously lived under reflection generation
- split ownership internally as:
  - `session-lessons` for lesson/cause/fix/prevention/stable decision/durable practice
  - `governance-candidates` for promotion-oriented governance outputs
  - artifact subtypes `follow-up-focus` / `next-turn-guidance` for downgraded derived/open-loop outputs

### Reflection layer
Responsibilities after refactor:
- recall/injection only
- no source loading endpoint for command-triggered generation in plugin runtime
- no job enqueue/status flow for command-triggered generation

## File-Level Change Plan

### TS/plugin runtime

#### `index.ts`
Remove:
- `runMemoryReflection` command-generation flow
- `command:new` / `command:reset` reflection hook registration
- duplicate-trigger bookkeeping used only for command reflection generation
- command-bound reflection source loading and enqueue logging paths

Retain/refactor carefully:
- `createReflectionPromptPlanner` usage if recall injection remains valuable
- `agent_end` transcript append path
- `distillCadenceState` and automatic distill enqueue path
- self-improvement bootstrap/local note behavior only where still meaningful independent of removed reflection generation

#### `src/backend-tools.ts`
Remove or deprecate in the same pass:
- `memory_reflection_status` if no reflection jobs remain

Retain:
- `memory_distill_enqueue`
- `memory_distill_status`
- debug recall tooling if still used for read paths

#### `src/backend-client/client.ts` and `src/backend-client/types.ts`
Remove unused client methods/types when corresponding reflection job surfaces are deleted from the runtime contract.
Keep only what is still needed for read-only reflection recall if applicable.

#### `src/context/reflection-prompt-planner.ts`
Retain as a read/injection module only.
Audit naming/comments so it does not imply ownership of generation or persistence.

### Tests

#### `test/remote-backend-shell-integration.test.mjs`
Delete or rewrite:
- `/new` reflection enqueue tests
- `/reset` reflection enqueue tests
- reflection enqueue non-blocking/failure cases

Retain and strengthen:
- automatic distill cadence tests
- distill management tool tests
- reflection recall fail-open tests only if recall remains

#### `test/memory-reflection.test.mjs`
Keep only recall/injection/session-state behavior that still exists after removal.
Delete coverage whose sole purpose was command-triggered reflection generation or handoff note coupling.

#### `backend/tests/phase2_contract_semantics.rs`
Rewrite contract coverage so backend distill owns the retained extraction semantics.
Remove endpoint contract tests for reflection job paths if those endpoints are deleted.
Keep or revise recall-only reflection tests based on surviving API shape.

## Backend Behavior Plan

### Turns-stage lesson extraction

Desired semantics:
- use ordered turns/messages from persisted transcript rows
- aggregate evidence across neighboring turns when building a lesson
- `session-lessons` produces and optionally persists:
  - Lesson
  - Cause
  - Fix
  - Prevention
  - stable decision / durable practice only when the evidence gate passes:
    - at least two distinct evidence messages; and
    - either repeated target phrasing across at least two messages or corroborating cause/fix/prevention context spanning at least two messages
  - stable decision / durable practice fall back to ordinary `Lesson` when that gate is not met
- `governance-candidates` produces:
  - worth-promoting learnings
  - skill extraction candidates
  - AGENTS/SOUL/TOOLS promotion candidates
- `Derived` / `Open loops / next actions` style outputs are downgraded into distill-owned artifact subtypes:
  - `follow-up-focus`
  - `next-turn-guidance`
- no retained reflection-generation output may bypass distill ownership

### Why not token-chunk map-stage
- token chunking is not the desired semantic unit
- the desired unit is turns/messages in conversational order
- internal reducer implementation can still have stages, but the public design language and tests must stay turn-based

## Verification Strategy

### TypeScript
- `npm test -- --test-name-pattern="distill|reflection|session-strategy"`

### Backend
- `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`

### Docs
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/turns-stage-distill-unification-2026-03-18`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/turns-stage-distill-unification-2026-03-18 README.md`

## Migration Notes

- No compatibility cleanup for command-triggered reflection generation.
- Delete stale code/tests/docs directly.
- Historical stored rows may remain in databases, but runtime code/docs should stop depending on them for generation semantics.

## Success Criteria

- one write path for trajectory-derived knowledge
- one trigger model for automatic generation (cadence via `everyTurns`)
- no `/new` / `/reset` generation path
- no public wording that confuses reflection generation with distill generation
