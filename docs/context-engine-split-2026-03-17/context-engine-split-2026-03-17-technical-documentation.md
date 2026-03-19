description: 2026-03-17 technical architecture snapshot for context-engine-split.
---

# context-engine-split Technical Documentation

Snapshot note:
- this document records the refreshed 2026-03-17 context-engine split design state;
- it reflects the current active file-layout judgment for context orchestration;
- `runtime-architecture.md` remains the top-level runtime/source-of-truth boundary document.
- older command-triggered generation wording from earlier snapshots is superseded by `runtime-architecture.md` and `docs/remote-memory-backend-2026-03-18/`.
- current canonical naming for prompt-time orchestration is `autoRecall` context + behavioral-guidance and `governance`; older transitional wording here is historical snapshot terminology only.

## Canonical Architecture

Current runtime contract remains:
- Plugin kind: `memory`
- Slot ownership: OpenClaw memory slot
- Public surfaces: memory tools, governance tools, current config schema

Current internal architecture in this snapshot:

1. **Backend/adapter core**
   - `src/backend-client/*`
   - `src/backend-tools.ts`
   - backend-owned retrieval, ranking, scope filtering, and persistence authority in the Rust backend

2. **Context orchestration core**
   - new `src/context/*` modules for prompt-time planning/rendering/state
   - consumes backend candidate providers and config
   - returns structured prompt prepend blocks and session-state updates
   - current active modules include:
     - `src/context/auto-recall-orchestrator.ts`
     - `src/context/behavioral-guidance-error-signals.ts`
     - `src/context/prompt-block-renderer.ts`
     - `src/context/session-exposure-state.ts`
     - `src/context/recall-engine.ts`
     - `src/context/adaptive-retrieval.ts`

3. **Plugin wiring layer**
   - `index.ts`
   - constructs dependencies, registers hooks/tools, delegates to backend/orchestration modules

## Key Constraints and Non-Goals

Constraints:
- Preserve the current `1.0.0-beta.0` `memory` plugin contract.
- Preserve gated hook paths:
  - `before_agent_start`
  - `before_prompt_build`
  - `after_tool_call`
  - `agent_end`
  - `session_end`
  - `before_reset`
- Preserve retrieval behavior and scopes.

Non-goals:
- No standalone ContextEngine shipping in this branch.
- No new persistent session transcript database.
- No DAG compaction or session-level archive features here.

## Interfaces between components

### Backend-facing interfaces (implemented in the current repo)
- Generic recall provider input:
  - query, actor identity, auto-recall limits
  - implementation seam: `src/context/auto-recall-orchestrator.ts` dependency contract (`recallGeneric`)
- Generic recall provider output:
  - backend-authoritative rows + rendered `prependContext` plan for `<relevant-memories>`
- Behavioral-guidance/error provider input:
  - query, actor identity, behavioral recall mode/limits, sessionKey
  - implementation seam: `src/context/auto-recall-orchestrator.ts` dependency contract (`recallBehavioral`, `sessionState`)
- Behavioral-guidance/error provider output:
  - rendered `prependContext` for `<behavioral-guidance>` and `<error-detected>`
  - session error-signal mutations via `src/context/session-exposure-state.ts` and extraction via `src/context/behavioral-guidance-error-signals.ts`

### Orchestration-facing interfaces (implemented in the current repo)
- Block renderer:
  - implementation seam: `src/context/prompt-block-renderer.ts`
  - output: tagged prompt blocks with untrusted-data wrapping where required
- Session state service:
  - implementation seam: `src/context/session-exposure-state.ts`
  - output: dynamic-recall suppression state + behavioral-guidance error-signal dedupe/TTL behavior
- Context orchestrators:
  - implementation seam: `src/context/auto-recall-orchestrator.ts`
  - output: hook-consumable plans returned to `index.ts`

## Operational behavior

Startup/bootstrap:
- `index.ts` parses config and constructs backend/orchestration dependencies.
- Hook registration remains in `index.ts`, and handlers are expected to stay thin delegates.

Runtime modes:
- `sessionStrategy: systemSessionMemory` keeps built-in OpenClaw session behavior and optional auto-recall.
- `sessionStrategy: autoRecall` enables the canonical behavioral autoRecall hook flows.
- `sessionStrategy: memoryReflection` is no longer supported; use `sessionStrategy: autoRecall`.
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
  - on failure logs an auto-recall warning and continues without injected block
- `after_tool_call` + `before_prompt_build`:
  - delegated to `createAutoRecallBehavioralPlanner(...)`
  - behavioral-guidance injection failures log a non-blocking warning; prompt build continues
- `session_end` + `before_reset`:
  - clear behavioral-guidance error state and dynamic recall session state
  - do not enqueue or generate trajectory-derived knowledge
  - failures remain non-blocking for surrounding runtime flow
- `agent_end`:
  - remains backend-owned auto-capture/store path in `index.ts` and backend modules

## Future thin-adapter handoff note

A later standalone ContextEngine adapter should stay thin and consume existing seams instead of re-implementing backend logic:
- Consume from current seams:
  - `createAutoRecallPlanner` for generic recall planning
  - `createAutoRecallBehavioralPlanner` for behavioral-guidance/error prompt planning
  - `createSessionExposureState` for session-local dedupe/suppression state
  - `prompt-block-renderer` helpers for `<relevant-memories>`, `<behavioral-guidance>`, `<error-detected>`
- Keep backend-owned in memory plugin:
  - LanceDB persistence and retrieval (`store.ts`, `retriever.ts`, `embedder.ts`)
  - scope resolution and access filtering (`scopes.ts`)
  - behavioral-guidance persistence/mapping over the stable backend memory contracts
  - tool registration + memory-slot contract (`tools.ts`, `openclaw.plugin.json`)
- During future contract migration, re-verify the same active hooks:
  - `before_agent_start`, `before_prompt_build`, `after_tool_call`, `agent_end`, `session_end`, `before_reset`

## Security model and hardening notes

- Scope filtering, retrieval ranking, and read authority remain backend-owned; orchestration must not reconstruct or bypass them locally.
- Rendered context blocks must keep existing untrusted-data wrapping semantics where applicable.
- No credential handling changes are in scope.
- Any future config edit outside this branch still requires backup of `openclaw.json` before runtime changes.

## Rollback and migration guardrails

- Compatibility baseline for this branch remains unchanged:
  - plugin kind stays `memory`
  - public tool names and config keys stay unchanged
- Rollback path for this refactor:
  - if orchestration regression is found, revert the `src/context/*` delegation wiring while keeping the same active hook surfaces (`before_agent_start`, `before_prompt_build`, `after_tool_call`, `agent_end`, `session_end`, `before_reset`)
- Migration safety note:
  - any future contract migration to a standalone ContextEngine must start from the extracted seams, then re-run active-hook parity tests before changing plugin contract exposure.

## Test strategy mapping

- Full regression: `npm test`
- Behavioral/governance-heavy paths: `node --test test/auto-recall-behavioral.test.mjs test/governance-tools.test.mjs`
- Session-strategy cutover guard: `node --test test/config-session-strategy-cutover.test.mjs`
- Doc hygiene: placeholder/residual scans under `docs/context-engine-split-2026-03-17`
