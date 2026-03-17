## Context

- Self-improvement support is still split between `src/self-improvement-files.ts` and `src/self-improvement-tools.ts`.
- The current split does not reflect a real ownership boundary: file bootstrap/append helpers exist only to serve the self-improvement tool surface and plugin startup path.
- The remaining `src/` root layout still mixes true root-level adapter/shared modules with prompt-context orchestration helpers and one thin remote-safe DTO file whose necessity is now questionable.

## Findings

- `index.ts` imports `ensureSelfImprovementLearningFiles` from `src/self-improvement-files.ts`.
- `src/self-improvement-tools.ts` imports `appendSelfImprovementEntry` and `ensureSelfImprovementLearningFiles` from `src/self-improvement-files.ts`.
- `test/self-improvement.test.mjs` imports from both modules.
- `src/memory-record-types.ts` is now referenced only by `src/context/auto-recall-orchestrator.ts`.
- The current `memory-record-types.ts` shape is a thin local DTO shell over backend rows and no longer carries active retrieval authority semantics.

## Goals / Non-goals

- Goals:
- Merge `src/self-improvement-files.ts` into `src/self-improvement-tools.ts`.
- Keep exported behavior stable for `ensureSelfImprovementLearningFiles`, `appendSelfImprovementEntry`, and `registerSelfImprovementTools`.
- Remove the extra module and update runtime/test imports.
- Record the context-boundary judgment for current `src/` root modules against `docs/context-engine-split-2026-03-17/`.
- Record whether `src/memory-record-types.ts` should remain, move, or be deleted.

- Non-goals:
- No behavior changes to self-improvement file formats or tool semantics.
- No new scope beyond the self-improvement module boundary cleanup.

## Target files / modules

- `src/self-improvement-files.ts`
- `src/self-improvement-tools.ts`
- `src/memory-record-types.ts`
- `src/context/auto-recall-orchestrator.ts`
- `src/context/recall-engine.ts`
- `src/context/adaptive-retrieval.ts`
- `index.ts`
- `test/self-improvement.test.mjs`
- `test/memory-reflection.test.mjs`

## Constraints

- Preserve current runtime behavior and test expectations.
- Keep `self-improvement-tools.ts` as the single public module for this area.

## Verification plan

- `npm test`
- `rg -n "self-improvement-files|appendSelfImprovementEntry|ensureSelfImprovementLearningFiles" src test index.ts`
- `rg -n "memory-record-types|RecallResultRow|MemoryEntry|RecallResultSources" src test index.ts`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/src-root-boundary-cleanup-2026-03-17`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/src-root-boundary-cleanup-2026-03-17 README.md`

## Rollback

- Restore `src/self-improvement-files.ts`.
- Repoint imports in `index.ts` and `test/self-improvement.test.mjs`.
- If `memory-record-types.ts` is later removed in this scope, restore it and rewire the local recall row typing in `src/context/auto-recall-orchestrator.ts`.

## Open questions

- None.

## Implementation status

- Completed: merged file bootstrap/append helpers into `src/self-improvement-tools.ts`.
- Completed: updated `index.ts` and `test/self-improvement.test.mjs` to import from the consolidated module.
- Completed: deleted `src/self-improvement-files.ts`.
- Completed: reviewed `src/` root files against the `context-engine-split` contract and classified which ones are appropriate `src/context/` candidates.
- Completed: localized the thin generic auto-recall row shape inside `src/context/auto-recall-orchestrator.ts` and deleted `src/memory-record-types.ts`.
- Completed: moved `src/recall-engine.ts` to `src/context/recall-engine.ts` and `src/adaptive-retrieval.ts` to `src/context/adaptive-retrieval.ts`, then updated runtime/test imports and active README path references.

## Evidence

- `npm test`
- `rg -n "self-improvement-files|appendSelfImprovementEntry|ensureSelfImprovementLearningFiles" src test index.ts`
- `rg -n "memory-record-types|RecallResultRow|MemoryEntry|RecallResultSources" src test index.ts`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/src-root-boundary-cleanup-2026-03-17`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/src-root-boundary-cleanup-2026-03-17 README.md`

## Context-boundary judgment

- Best fit for `src/context/`:
- `src/context/recall-engine.ts`
  - Reason: owns prompt-time gating, session dedupe, exposure-state-driven injection, and prompt block assembly support; this matches the orchestration boundary described in `context-engine-split`.
- `src/context/adaptive-retrieval.ts`
  - Reason: owns prompt-side "should retrieval run at all" heuristics and is part of the orchestration path rather than backend authority.

- Optional / depends on future shared-types direction:
- `src/memory-record-types.ts`
  - Updated judgment: the file did not need to survive as a standalone root module. Its final active use was a historical local DTO shell over backend-returned rows.
  - Implementation result: deleted in this scope after localizing the thin row shape in `src/context/auto-recall-orchestrator.ts`.

- Not appropriate for `src/context/`:
- `src/backend-tools.ts`
  - Reason: backend tool registration and adapter surface, not prompt-time context planning/rendering.
- `src/self-improvement-tools.ts`
  - Reason: governance/file/tool lifecycle for self-improvement, not the memory-context injection contract.
