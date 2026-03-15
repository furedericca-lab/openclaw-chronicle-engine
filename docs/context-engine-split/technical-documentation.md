---
description: Canonical technical architecture for context-engine-split.
---

# context-engine-split Technical Documentation

## Canonical Architecture

Current runtime contract remains:
- Plugin kind: `memory`
- Slot ownership: OpenClaw memory slot
- Public surfaces: memory tools, reflection/self-improvement tools, current config schema

Target internal architecture after this branch:

1. **Backend memory core**
   - `src/store.ts`
   - `src/embedder.ts`
   - `src/retriever.ts`
   - `src/scopes.ts`
   - `src/reflection-store.ts`
   - `src/tools.ts`

2. **Context orchestration core**
   - new `src/context/*` modules for prompt-time planning/rendering/state
   - consumes backend candidate providers and config
   - returns structured prompt prepend blocks and session-state updates

3. **Plugin wiring layer**
   - `index.ts`
   - constructs dependencies, registers hooks/tools, delegates to backend/orchestration modules

## Key Constraints and Non-Goals

Constraints:
- Preserve current `memory` plugin contract and config compatibility.
- Preserve gated hook paths:
  - `before_agent_start`
  - `before_prompt_build`
  - `after_tool_call`
  - `agent_end`
  - `command:new`
  - `command:reset`
- Preserve retrieval behavior and scopes.

Non-goals:
- No standalone ContextEngine shipping in this branch.
- No new persistent session transcript database.
- No DAG compaction or session-level archive features here.

## Interfaces between components

### Backend-facing interfaces (implemented in this branch)
- Generic recall provider input:
  - query, actor identity, auto-recall limits
  - implementation seam: `src/context/auto-recall-orchestrator.ts` dependency contract (`recallGeneric`)
- Generic recall provider output:
  - backend-authoritative rows + rendered `prependContext` plan for `<relevant-memories>`
- Reflection/error provider input:
  - query, actor identity, reflection recall mode/limits, sessionKey
  - implementation seam: `src/context/reflection-prompt-planner.ts` dependency contract (`recallReflection`, `sessionState`)
- Reflection/error provider output:
  - rendered `prependContext` for `<inherited-rules>` and `<error-detected>`
  - session error-signal mutations via `src/context/session-exposure-state.ts`

### Orchestration-facing interfaces (implemented in this branch)
- Block renderer:
  - implementation seam: `src/context/prompt-block-renderer.ts`
  - output: tagged prompt blocks with untrusted-data wrapping where required
- Session state service:
  - implementation seam: `src/context/session-exposure-state.ts`
  - output: dynamic-recall suppression state + reflection error-signal dedupe/TTL behavior
- Context orchestrators:
  - implementation seams: `src/context/auto-recall-orchestrator.ts`, `src/context/reflection-prompt-planner.ts`
  - output: hook-consumable plans returned to `index.ts`

## Operational behavior

Startup/bootstrap:
- `index.ts` parses config and constructs backend/orchestration dependencies.
- Hook registration remains in `index.ts`, but handlers should become thin delegates.

Runtime modes:
- `sessionStrategy: systemSessionMemory` keeps built-in OpenClaw session behavior and optional auto-recall.
- `sessionStrategy: memoryReflection` enables reflection-specific hook flows.
- `sessionStrategy: none` disables session strategy hooks while preserving memory backend availability.

Prompt-time data flow after refactor:
1. Hook handler receives event.
2. Handler asks orchestration layer whether recall/injection should run.
3. Orchestration layer fetches backend-authoritative rows from adapter/provider modules.
4. Renderer produces block text.
5. Hook returns prepend payload.

## Observability and error handling

- Hook-level failures remain fail-open with explicit warnings in plugin logs.
- Error-signal collection remains available so non-trivial tool failures can still surface as `<error-detected>` guidance.
- Documentation and tests must specify when a failure is backend retrieval vs orchestration/rendering.

### Hook parity and fail-open wording (current behavior)
- `before_agent_start`:
  - delegated to `createAutoRecallPlanner(...).plan(...)`
  - on failure logs `memory-lancedb-pro: auto-recall failed: ...` and continues without injected block
- `after_tool_call` + `before_prompt_build`:
  - delegated to `createReflectionPromptPlanner(...)`
  - reflection-recall injection failures log `memory-reflection: reflection-recall injection failed: ...`; prompt build continues
- `command:new` / `command:reset`:
  - reflection hook keeps durable registration + duplicate-trigger guard
  - trigger is normalized to shared contract values (`new` / `reset`) before local-vs-remote branching
  - post-hook session clear now applies to both reflection error signals and dynamic recall state
  - failures log `memory-reflection: hook failed: ...`; command flow continues
- `agent_end`:
  - remains backend-owned auto-capture/store path in `index.ts` and backend modules

## Future thin-adapter handoff note

A later standalone ContextEngine adapter should stay thin and consume existing seams instead of re-implementing backend logic:
- Consume from current seams:
  - `createAutoRecallPlanner` for generic recall planning
  - `createReflectionPromptPlanner` for reflection/error prompt planning
  - `createSessionExposureState` for session-local dedupe/suppression state
  - `prompt-block-renderer` helpers for `<relevant-memories>`, `<inherited-rules>`, `<error-detected>`
- Keep backend-owned in memory plugin:
  - LanceDB persistence and retrieval (`store.ts`, `retriever.ts`, `embedder.ts`)
  - scope resolution and access filtering (`scopes.ts`)
  - reflection persistence/mapping (`reflection-store.ts` and related reflection backends)
  - tool registration + memory-slot contract (`tools.ts`, `openclaw.plugin.json`)
- During future contract migration, re-verify the same active hooks:
  - `before_agent_start`, `before_prompt_build`, `after_tool_call`, `agent_end`, `command:new`, `command:reset`

## Security model and hardening notes

- Scope filtering and read authority remain backend-owned; orchestration must not reconstruct or bypass them locally.
- Rendered context blocks must keep existing untrusted-data wrapping semantics where applicable.
- No credential handling changes are in scope.
- Any future config edit outside this branch still requires backup of `openclaw.json` before runtime changes.

## Rollback and migration guardrails

- Compatibility baseline for this branch remains unchanged:
  - plugin kind stays `memory`
  - public tool names and config keys stay unchanged
- Rollback path for this refactor:
  - if orchestration regression is found, revert the `src/context/*` delegation wiring while keeping the same active hook surfaces (`before_agent_start`, `before_prompt_build`, `after_tool_call`, `agent_end`, `command:new`, `command:reset`)
- Migration safety note:
  - any future contract migration to a standalone ContextEngine must start from the extracted seams, then re-run active-hook parity tests before changing plugin contract exposure.

## Test strategy mapping

- Full regression: `npm test`
- Reflection-heavy paths: `node --test test/memory-reflection.test.mjs test/self-improvement.test.mjs`
- Config compatibility: `node test/config-session-strategy-migration.test.mjs`
- Doc hygiene: placeholder/residual scans under `docs/context-engine-split`
