description: 2026-03-17 brainstorming snapshot for context-engine-split.
---

# context-engine-split Brainstorming

Snapshot note:
- this file captures the refreshed 2026-03-17 decision framing after the backend-authority and `1.0.0-beta.0` cutover work;
- it is kept as design context for current module placement, not as the canonical runtime contract.
- any older mention of command-triggered reflection generation is superseded by `../runtime-architecture.md` and `../remote-memory-backend-2026-03-18/`.
- current canonical naming for prompt-time orchestration is `autoRecall` context + behavioral-guidance and `governance`; older reflection/self-improvement wording in this snapshot is historical only.

## Problem

`openclaw-chronicle-engine` still needs a cleaner boundary between backend/adapter work and turn-time context orchestration. The backend side is already Rust-owned for retrieval/ranking/scope authority, while the plugin still owns prompt gating, `<relevant-memories>` injection, `<behavioral-guidance>` injection, `<error-detected>` injection, and session-local dedupe/suppression/cleanup state. This makes the plugin harder to evolve toward a dedicated ContextEngine path unless the remaining orchestration files are kept visibly separated from backend/adapter modules.

Desired outcome: keep `openclaw-chronicle-engine` as a strong backend-authoritative memory plugin while keeping prompt-time orchestration behind a thin internal seam that could later move into a separate ContextEngine implementation. Success means backend retrieval/tool behavior stays stable, prompt orchestration stays modular and visibly grouped, and migration can be validated by active-path tests before any external contract switch.

## Scope

In scope:
- Separate backend row retrieval/authority from prompt/context assembly responsibilities.
- Introduce internal seams for generic auto-recall, behavioral-guidance recall, and error-signal prompt hints.
- Reduce `index.ts` ownership of prompt orchestration by moving logic into dedicated modules that are ContextEngine-ready.
- Preserve current public plugin kind (`memory`) and current config schema during this refactor.
- Add docs and tests that make later extraction to a standalone ContextEngine auditable.

Out of scope:
- Shipping a brand-new OpenClaw ContextEngine plugin in this branch.
- Replacing backend-owned retrieval, ranking, scope, or tool APIs.
- Changing session cleanup behavior beyond internal seam extraction.
- Adding lossless transcript DAG/session compaction in this branch.

## Constraints

- Framework reality first: this repository currently implements `kind: "memory"` in `openclaw.plugin.json`; no fake contract migration is allowed in this branch.
- Active paths must stay working: `before_agent_start`, `before_prompt_build`, `after_tool_call`, `agent_end`, `session_end`, and `before_reset`.
- Storage/retrieval concerns must remain in backend-owned Rust services and the plugin's backend adapter modules such as `src/backend-client/*` and `src/backend-tools.ts`.
- Prompt-time exposure decisions must move toward orchestration modules without changing user-facing config keys.
- Tests must cover unset-vs-set config behavior and hook-driven paths before any future default-path switch.

## Options

### Option A — Keep everything in `index.ts`, only add comments/docs
- Complexity: low.
- Migration impact: none.
- Reliability: poor long-term; mixed responsibilities remain.
- Rollback: trivial.
- Rejected because it does not create reusable seams for ContextEngine migration.

### Option B — Internal seam extraction only, keep plugin kind and hook wiring unchanged
- Complexity: medium.
- Migration impact: low.
- Reliability: strong if tests cover behavior parity.
- Rollback: low-risk because external contracts stay stable.
- Good first step because it decouples backend-owned retrieval from prompt rendering/exposure.

### Option C — Directly convert this plugin from `memory` to `contextEngine`
- Complexity: high.
- Migration impact: high.
- Reliability: risky because current behavior depends on hook paths and memory-slot semantics.
- Rollback: expensive.
- Rejected because it conflates architecture ideal with current framework contract.

### Option D — Create a second ContextEngine plugin immediately and split behavior across two repos at once
- Complexity: high.
- Migration impact: medium/high.
- Reliability: medium if done well, but coordination cost is high.
- Rollback: moderate.
- Deferred: appropriate after Option B lands and stabilizes the provider/adapter seams.

## Decision

Choose **Option B** in this branch.

This branch should aggressively extract prompt orchestration out of `index.ts` and into dedicated internal modules while preserving the existing `memory` plugin contract. The new module boundaries should make a later thin ContextEngine adapter straightforward, but this branch must not pretend that adapter already exists.
For v1, `agent_end` remains in the backend branch as ingestion logic; any further abstraction is deferred and not part of the current frozen contract.

## Risks

- Hook parity risk: moving orchestration code may subtly change injection order or dedupe behavior.
- Config compatibility risk: `autoRecallSelectionMode`, `autoRecallBehavioral.recall.*`, and legacy `memoryReflection.recall.*` alias mapping must remain unchanged.
- Test blind spots: if session cleanup or dynamic reflection paths are not re-verified, the refactor can silently regress.
- Documentation drift: README architecture sections must not overclaim a shipped ContextEngine.

## Open Questions

- Which exact orchestration surfaces should consume backend-authoritative rows directly vs renderer-owned text blocks in this branch?
- Do we want a future `context-engine-memory-orchestrator` inside this repo or as a sibling repo/plugin?
