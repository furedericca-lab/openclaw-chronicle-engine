---
description: Implementation research notes for context-engine-split.
---

# context-engine-split Implementation Research Notes

## Problem statement and current baseline

Current plugin contract:
- `openclaw.plugin.json` declares `"kind": "memory"`.
- `index.ts` exports `kind: "memory" as const` and registers the whole plugin.

Current backend-heavy modules:
- `src/store.ts` — LanceDB storage and query primitives.
- `src/embedder.ts` — embedding provider abstraction.
- `src/retriever.ts` — hybrid retrieval, scoring, rerank.
- `src/scopes.ts` — scope access model.
- `src/reflection-store.ts` / `src/reflection-item-store.ts` / `src/reflection-event-store.ts` — reflection persistence.
- `src/tools.ts` — memory/self-improvement tool registration.

Current orchestration-heavy ownership in `index.ts`:
- `before_agent_start` generic auto-recall injection (`index.ts:1675-1715`).
- `agent_end` auto-capture (`index.ts:1718+`).
- durable hook registration for `/new` and `/reset` (`index.ts:1849+`, `1981+`, `2495+`).
- `after_tool_call` error-signal collection (`index.ts:2063+`).
- `before_prompt_build` for `<inherited-rules>` and `<error-detected>` injection (`index.ts:2150-2182`).
- parsing of reflection/auto-recall config in `parsePluginConfig` (`index.ts:2722+`).

Supporting orchestration helpers already exist, but are mixed with plugin wiring:
- `src/recall-engine.ts` — prompt gating/session dedupe/tagged block assembly helper.
- `src/auto-recall-final-selection.ts` — generic final top-k selection adapter.
- `src/reflection-recall.ts` / `src/reflection-aggregation.ts` / `src/reflection-recall-final-selection.ts` — reflection candidate ranking.
- `src/adaptive-retrieval.ts` — query worthiness heuristics.
- `src/session-recovery.ts` / `src/reflection-retry.ts` — command-flow runtime helpers.

## Gap analysis with evidence

1. **Storage/retrieval and context exposure are mixed in the plugin entrypoint.**
   Evidence: `index.ts` both constructs backend objects (`MemoryStore`, `embedder`, `retriever`, `scopeManager`) and directly renders prompt blocks for `<relevant-memories>`, `<inherited-rules>`, and `<error-detected>`.

2. **Prompt-time state is kept alongside backend setup.**
   Evidence: `autoRecallState`, `reflectionErrorStateBySession`, and reflection-agent caches are all created in `index.ts`, even though these are session exposure concerns rather than persistence primitives.

3. **Current recall helpers are reusable but not presented as explicit provider contracts.**
   Evidence: `orchestrateDynamicRecall()` in `src/recall-engine.ts` accepts loader/formatter lambdas, but callers still define prompt tags and rendering directly in `index.ts`.

4. **Future ContextEngine migration lacks a thin adapter seam.**
   Evidence: no module currently exposes a backend-facing API like "get auto-recall candidates" or "get reflection hint candidates" without also deciding output tags and block formatting.

5. **Hook-driven behavior is gated and cannot be assumed safe to move blindly.**
   Evidence: tests in `test/memory-reflection.test.mjs` and `test/config-session-strategy-migration.test.mjs` cover session strategy, dynamic reflection recall, and selection-mode compatibility. These paths must remain green before any later contract change.

## Architecture/implementation options and trade-offs

### Option 1 — Minimal doc-only refactor
- Update README/docs to describe desired architecture, no code seam extraction.
- Low cost, but does not reduce `index.ts` coupling.
- Not sufficient for Codex execution against concrete refactor goals.

### Option 2 — Provider/adapter extraction inside existing plugin (selected)
- Add internal modules for candidate production and prompt block rendering.
- Keep current hook registration and `memory` contract.
- Enables future ContextEngine adapter with low migration risk.
- Requires moderate movement of logic and targeted tests.

### Option 3 — Immediate dual-plugin split (`memory` + `contextEngine`) in one branch
- Architecturally clean end-state.
- Too risky without first validating provider seams and hook parity.
- Higher review burden and larger rollback scope.

## Selected design and rationale

Selected design: **extract a ContextEngine-ready orchestration layer inside the current repo while preserving the public `memory` plugin contract.**

Planned module boundary shift:
- Keep backend candidate production in backend-oriented modules.
- Introduce orchestration modules that own:
  - recall exposure planning,
  - reflection exposure planning,
  - error-signal exposure planning,
  - prompt block rendering,
  - session-local exposure suppression state.
- Keep `index.ts` responsible only for plugin bootstrap, dependency wiring, and hook registration.
- Ensure orchestration modules return structured blocks/plan objects rather than directly touching backend persistence internals.

Proposed seam modules:
- `src/context/auto-recall-provider.ts` — fetch/filter/prepare generic recall candidates from backend.
- `src/context/reflection-provider.ts` — fetch/aggregate reflection recall candidates.
- `src/context/error-signal-provider.ts` — expose pending tool-error hints for prompt-time use.
- `src/context/block-renderer.ts` — render `<relevant-memories>`, `<inherited-rules>`, `<error-detected>` from structured inputs.
- `src/context/session-state.ts` — session-local suppression/dedupe state previously owned in `index.ts`.
- Optional thin composition module: `src/context/context-orchestrator.ts`.

## Test and validation strategy

Primary repo command:
- `npm test`

Focused path checks:
- `node --test test/memory-reflection.test.mjs test/self-improvement.test.mjs`
- `node test/config-session-strategy-migration.test.mjs`

Documentation/residual checks:
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/context-engine-split`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/context-engine-split README.md`

Expected outcomes:
- No behavior change in existing memory slot/tool behavior.
- Same config parsing results for `autoRecallSelectionMode`, `sessionStrategy`, and `memoryReflection.recall.*`.
- Same hook-time injection semantics from the test suite point of view.

## Risks, assumptions, unresolved questions

Risks:
- Injection ordering drift between `<inherited-rules>` and `<error-detected>`.
- Hidden coupling to local helper closures in `index.ts`.
- README architecture table may need careful updates to avoid claiming a shipped standalone ContextEngine.

Assumptions:
- OpenClaw current memory-plugin hooks remain available and stable in this branch.
- A later standalone ContextEngine can consume the extracted orchestration/provider modules with only thin glue.

Unresolved questions:
- Whether `agent_end` auto-capture should remain entirely outside the new orchestration directory.
- Whether the eventual ContextEngine should live in this repository or a sibling plugin package.
