---
description: Brainstorming and decision framing for autorecall-governance-unification-2026-03-18.
---

# autorecall-governance-unification-2026-03-18 Brainstorming

## Problem
- Prompt-time recall/injection is still split across generic auto-recall in `src/context/auto-recall-orchestrator.ts` and a separate reflection planner in `src/context/reflection-prompt-planner.ts`.
- Governance backlog tooling still ships as `self_improvement_*` in `src/self-improvement-tools.ts`, while bootstrap/reminder behavior is coupled to the same self-improvement concept in `index.ts`.
- Active docs/config/tests still present `memoryReflection` and `selfImprovement` as first-class runtime concepts even though `distill` already owns trajectory-derived generation and `governance-candidates` already owns promotion-oriented artifacts.

## Scope
- Reframe former reflection recall/injection as an internal autoRecall behavioral-guidance channel.
- Re-home backlog/review/extract/promotion surfaces under governance naming and ownership.
- Continue removing reflection-generation-era naming from active plugin/docs/tests/config while preserving backend wire compatibility where renaming the Rust contract is out of scope.

## Constraints
- `distill` remains the only trajectory-derived generation authority.
- Active runtime behavior should stay fail-open for prompt-time recall/guidance injection failures.
- Backend HTTP contracts remain stable for this scope; plugin-side adapters may map public behavioral naming onto existing backend reflection endpoints.
- Any compatibility aliases must be explicitly documented as transitional.

## Options
### Option A: Docs-only rename, keep runtime split
- Keep `createReflectionPromptPlanner`, `memoryReflection`, and `selfImprovement` in active code.
- Rewrite README/runtime docs to say autoRecall and governance own the concepts.
- Reject because the active runtime surface would still preserve reflection/self-improvement as peer top-level concepts.

### Option B: Unify prompt-time behavior under autoRecall and split governance ownership
- Replace the dedicated reflection planner with an autoRecall behavioral-guidance path.
- Replace `self_improvement_*` tools with governance-owned tool names, while moving reminder/bootstrap note behavior under behavioral autoRecall guidance.
- Keep backend `/v1/recall/reflection` and category=`reflection` only as adapter-level implementation details.
- Accept because it matches the target architecture without requiring a risky backend contract rewrite.

### Option C: Rename the full backend wire contract in the same scope
- Rename Rust endpoints, DB fields, backend traces, and plugin adapters away from `reflection`.
- Reject for this scope because it expands blast radius into backend persistence/API parity without adding user-visible architectural value beyond what adapter-level reframing already provides.

## Decision
- Choose Option B.
- Canonical public prompt-time architecture becomes:
  - autoRecall context channel: generic `<relevant-memories>` injection.
  - autoRecall behavioral-guidance channel: behavioral recall plus recent tool-error reminders in `<behavioral-guidance>` / `<error-detected>`.
- Canonical backlog/workflow architecture becomes governance:
  - `governance_log`
  - `governance_review`
  - `governance_extract_skill`
  - governance backlog files under a governance-owned path, with legacy `.learnings` treated as compatibility input only.
- Transitional compatibility is allowed for:
  - `sessionStrategy: "memoryReflection"` as an alias for the new autoRecall behavioral mode.
  - `memoryReflection` config as an alias for the new behavioral autoRecall config.
  - `selfImprovement` config as an alias split across governance and behavioral autoRecall reminder settings.
  - `self_improvement_*` tools as aliases to governance tools when required for compatibility.

## Risks
- Prompt-tag renaming from `<inherited-rules>` to `<behavioral-guidance>` can break tests or downstream prompt assertions if not updated coherently.
- Governance file migration from `.learnings` to a governance-owned directory can orphan existing backlog data unless the runtime reads legacy files or copies them forward.
- Session strategy/config alias handling can silently drift if parser defaults and UI schema do not move together.
- Debug tooling still touches backend `reflection` traces; docs must keep that implementation detail demoted instead of presenting it as a public architecture peer.

## Open Questions
- Whether `memory_recall_debug` should expose public `behavioral` channel naming immediately or keep legacy `reflection` channel terminology for one transition window.
- Whether the runtime should physically copy legacy `.learnings/*` files into the new governance path or prefer read-through compatibility only.
