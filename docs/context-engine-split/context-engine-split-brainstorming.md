---
description: Brainstorming and decision framing for context-engine-split.
---

# context-engine-split Brainstorming

## Problem

`memory-lancedb-pro` currently mixes two concerns in `index.ts`: long-term memory backend work (LanceDB storage, embedding, retrieval, reflection persistence, tool registration) and turn-time context orchestration (prompt gating, `<relevant-memories>` injection, `<inherited-rules>` injection, `<error-detected>` injection, session-local dedupe/suppression state, and `/new`/`/reset` reflection flows). This makes the plugin harder to evolve toward a dedicated ContextEngine path without destabilizing memory retrieval behavior.

Desired outcome: keep `memory-lancedb-pro` as a strong memory backend while extracting prompt-time orchestration behind a thin compatibility layer that can later move into a separate ContextEngine implementation. Success means backend retrieval/tool behavior stays stable, prompt orchestration becomes modular, and migration can be validated by active-path tests before any external contract switch.

## Scope

In scope:
- Separate backend row retrieval/authority from prompt/context assembly responsibilities.
- Introduce internal seams for generic auto-recall, reflection recall, and error-signal prompt hints.
- Reduce `index.ts` ownership of prompt orchestration by moving logic into dedicated modules that are ContextEngine-ready.
- Preserve current public plugin kind (`memory`) and current config schema during this refactor.
- Add docs and tests that make later extraction to a standalone ContextEngine auditable.

Out of scope:
- Shipping a brand-new OpenClaw ContextEngine plugin in this branch.
- Replacing LanceDB storage, retrieval scoring, rerank providers, scopes, or tool APIs.
- Changing `/new` or `/reset` external behavior beyond internal seam extraction.
- Adding lossless transcript DAG/session compaction in this branch.

## Constraints

- Framework reality first: this repository currently implements `kind: "memory"` in `openclaw.plugin.json`; no fake contract migration is allowed in this branch.
- Active paths must stay working: `before_agent_start`, `before_prompt_build`, `after_tool_call`, `agent_end`, `command:new`, and `command:reset`.
- Storage/retrieval concerns must remain in backend modules such as `src/store.ts`, `src/embedder.ts`, `src/retriever.ts`, `src/reflection-store.ts`, and `src/tools.ts`.
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
- Config compatibility risk: `autoRecallSelectionMode`, `memoryReflection.recall.*`, and session-strategy behavior must remain unchanged.
- Test blind spots: if `/new`/`/reset` or dynamic reflection paths are not re-verified, the refactor can silently regress.
- Documentation drift: README architecture sections must not overclaim a shipped ContextEngine.

## Open Questions

- Which exact orchestration surfaces should consume backend-authoritative rows directly vs renderer-owned text blocks in this branch?
- Do we want a future `context-engine-memory-orchestrator` inside this repo or as a sibling repo/plugin?
