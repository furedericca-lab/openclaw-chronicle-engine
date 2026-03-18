---
description: Implementation research notes for context-engine-split.
---

# context-engine-split Implementation Research Notes

## Problem statement and current baseline

Current plugin contract:
- `openclaw.plugin.json` declares `"kind": "memory"`.
- `index.ts` exports `kind: "memory" as const` and registers the whole plugin.

Current backend/adapter-heavy modules:
- `src/backend-client/*` — backend transport/runtime-context boundary.
- `src/backend-tools.ts` — memory and management tool registration against the backend client.
- Rust backend — retrieval, ranking, rerank, scope visibility, and persistence authority.

Current orchestration-heavy ownership in `index.ts`:
- `before_agent_start` generic auto-recall injection (`index.ts:1675-1715`).
- `agent_end` auto-capture (`index.ts:1718+`).
- `after_tool_call` error-signal collection (`index.ts:2063+`).
- `before_prompt_build` for `<behavioral-guidance>` and `<error-detected>` injection (`index.ts:2150-2182` in the snapshot; canonical naming later changed in follow-up scopes).
- `session_end` + `before_reset` cleanup for behavioral-guidance prompt/session state (`index.ts:1113+`).
- parsing of behavioral autoRecall and generic autoRecall config in `parsePluginConfig` (`index.ts:2722+`).

Supporting orchestration helpers already exist and are now largely under `src/context/`:
- `src/context/recall-engine.ts` — prompt gating/session dedupe/tagged block assembly helper.
- `src/context/adaptive-retrieval.ts` — query worthiness heuristics.
- `src/context/auto-recall-orchestrator.ts` — generic auto-recall planner.
- `src/context/reflection-prompt-planner.ts` — historical name for the behavioral-guidance/error planner seam.
- `src/context/session-exposure-state.ts` — session-local exposure suppression and error-signal state.

## Gap analysis with evidence

1. **Storage/retrieval and context exposure are mixed in the plugin entrypoint.**
   Evidence: `index.ts` both constructs backend objects (`MemoryStore`, `embedder`, `retriever`, `scopeManager`) and directly renders prompt blocks for `<relevant-memories>`, `<behavioral-guidance>`, and `<error-detected>`.

2. **Prompt-time state is kept alongside backend setup.**
   Evidence: `autoRecallState`, `reflectionErrorStateBySession`, and reflection-agent caches are all created in `index.ts`, even though these are session exposure concerns rather than persistence primitives.

3. **The reusable recall helpers are now context-owned, but the design docs need to track the moved paths precisely.**
   Evidence: `orchestrateDynamicRecall()` now lives in `src/context/recall-engine.ts`, while callers in `src/context/*` consume it directly.

4. **Future ContextEngine migration lacks a thin adapter seam.**
   Evidence: no module currently exposes a backend-facing API like "recall generic rows" or "recall reflection rows" without also deciding output tags and block formatting.

5. **Hook-driven behavior is gated and cannot be assumed safe to move blindly.**
   Evidence: tests in `test/auto-recall-behavioral.test.mjs` and `test/config-session-strategy-cutover.test.mjs` cover session strategy, dynamic behavioral recall, and selection-mode behavior. These paths must remain green before any later contract change.

## Architecture/implementation options and trade-offs

### Option 1 — Minimal doc-only refactor
- Update README/docs to describe desired architecture, no code seam extraction.
- Low cost, but does not reduce `index.ts` coupling.
- Not sufficient for Codex execution against concrete refactor goals.

### Option 2 — Provider/adapter extraction inside existing plugin (selected)
- Add internal modules for backend-row access and prompt block rendering.
- Keep current hook registration and `memory` contract.
- Enables future ContextEngine adapter with low migration risk.
- Requires moderate movement of logic and targeted tests.

### Option 3 — Immediate dual-plugin split (`memory` + `contextEngine`) in one branch
- Architecturally clean end-state.
- Too risky without first validating adapter/orchestration seams and hook parity.
- Higher review burden and larger rollback scope.

## Selected design and rationale

Selected design: **keep the ContextEngine-ready orchestration layer inside the current repo while preserving the public `memory` plugin contract.**

Planned module boundary shift:
- Keep backend-owned retrieval/persistence in backend-oriented modules.
- Introduce orchestration modules that own:
  - recall exposure planning,
  - behavioral-guidance exposure planning,
  - error-signal exposure planning,
  - prompt block rendering,
  - session-local exposure suppression state.
- Keep `index.ts` responsible only for plugin bootstrap, dependency wiring, and hook registration.
- Ensure orchestration modules return structured blocks/plan objects rather than directly touching backend persistence internals.
- Remove local scope participation from orchestration contracts; orchestration should consume backend-authoritative rows and decide only timing/rendering.
- For v1, `agent_end` remains in the backend/data path as the auto-capture ingestion boundary; further abstraction is explicitly deferred.

Current seam modules:
- backend data access:
  - `src/backend-client/client.ts`
  - `src/backend-client/runtime-context.ts`
- context orchestration:
  - `src/context/auto-recall-orchestrator.ts`
  - `src/context/reflection-prompt-planner.ts` (historical filename; compatibility shim in later scopes)
  - `src/context/prompt-block-renderer.ts`
  - `src/context/session-exposure-state.ts`
  - `src/context/recall-engine.ts`
  - `src/context/adaptive-retrieval.ts`

## Test and validation strategy

Primary repo command:
- `npm test`

Focused path checks:
- `node --test test/auto-recall-behavioral.test.mjs test/governance-tools.test.mjs`
- `node --test test/config-session-strategy-cutover.test.mjs`

Documentation/residual checks:
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/context-engine-split-2026-03-17`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/context-engine-split-2026-03-17 README.md`

Expected outcomes:
- No behavior change in existing memory slot/tool behavior.
- Same config parsing results for `autoRecallSelectionMode`, `sessionStrategy`, `autoRecallBehavioral.recall.*`, and legacy `memoryReflection.recall.*` alias mapping.
- Same hook-time injection semantics from the test suite point of view.

## Risks, assumptions, unresolved questions

Risks:
- Injection ordering drift between `<behavioral-guidance>` and `<error-detected>`.
- Hidden coupling to local helper closures in `index.ts`.
- Residual local scope-filter assumptions surviving inside orchestration/adapter seams.
- README architecture table may need careful updates to avoid claiming a shipped standalone ContextEngine.

Assumptions:
- OpenClaw current memory-plugin hooks remain available and stable in this branch.
- A later standalone ContextEngine can consume the extracted orchestration/adapter modules with only thin glue.

Unresolved questions:
- Whether the eventual ContextEngine should live in this repository or a sibling plugin package.
