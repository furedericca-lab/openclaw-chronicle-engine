---
description: Implementation research notes for ts-residual-debt-cleanup-2026-03-17.
---

# ts-residual-debt-cleanup-2026-03-17 Implementation Research Notes

## Baseline (Current State)

Canonical runtime architecture today:

- backend authority lives in `backend/src/*`;
- adapter/runtime wiring lives in `index.ts`, `src/backend-client/*`, and `src/backend-tools.ts`;
- prompt-time orchestration lives in `src/context/*`.

Current retained TS recall/reflection helper map after Phase 2/3 cleanup:

| Path | Current observed use | Final classification |
| --- | --- | --- |
| `src/recall-engine.ts` | imported by `src/context/auto-recall-orchestrator.ts`, `src/context/reflection-prompt-planner.ts`, `src/context/session-exposure-state.ts`, and test files | production prompt-local helper |
| `src/adaptive-retrieval.ts` | imported by `src/recall-engine.ts` and tests | production prompt-local utility |
| `src/prompt-local-auto-recall-selection.ts` | imported by `src/context/auto-recall-orchestrator.ts` and tests | production prompt-local post-selection seam |
| `src/prompt-local-topk-setwise-selection.ts` | imported by `src/prompt-local-auto-recall-selection.ts` and `test/helpers/reflection-recall-selection-reference.ts` | shared prompt-local utility with clearer ownership |
| `test/helpers/reflection-recall-reference.ts` | imported by `test/memory-reflection.test.mjs`; no production imports found | test/reference helper |
| `test/helpers/reflection-recall-selection-reference.ts` | imported only by `test/helpers/reflection-recall-reference.ts` | test/reference helper downstream utility |

Historical local-authority modules already removed from the repo:

- `src/store.ts`
- `src/retriever.ts`
- `src/embedder.ts`
- `src/tools.ts`
- `src/migrate.ts`
- `src/scopes.ts`
- `src/access-tracker.ts`
- `cli.ts`

## Gap Analysis

1. **There is little evidence of surviving TS authority logic, but there was naming debt.**
   - `src/auto-recall-final-selection.ts` and `src/final-topk-setwise-selection.ts` sounded like backend ranking modules.
   - They have now been renamed to `src/prompt-local-auto-recall-selection.ts` and `src/prompt-local-topk-setwise-selection.ts` to reflect their actual seam ownership.

2. **The reflection reference helpers were the strongest “looks-live-but-is-not-production” debt.**
   - Import evidence showed test usage but no active production path.
   - They have now been moved under `test/helpers/` so they no longer read like runtime authority modules.

3. **Top-level `src/` no longer mixes runtime-critical seams with reflection reference helpers.**
   - This reduces onboarding ambiguity.
   - It also reduces future accidental reuse of reference helpers in production code.

4. **The remaining debt is narrow naming/placement polish, not missing migration.**
   - The old authority chain is already gone.
   - The repo now presents the remaining local code as prompt-local seams rather than unfinished authority migration residue.

## Candidate Designs and Trade-offs

### Option 1: docs-only clarification

Pros:

- no code movement;
- minimal risk.

Cons:

- leaves confusing file locations intact;
- future maintainers still see top-level `src/reflection-recall.ts` and ask whether production depends on it.

### Option 2: move only test/reference helpers out of top-level `src/`

Pros:

- highest signal-to-risk ratio;
- reduces the most misleading artifacts first.

Cons:

- does not address naming debt in production prompt-local modules.

### Option 3: two-step cleanup

Step A:

- move test/reference-only helpers out of top-level `src/`.

Step B:

- rename production prompt-local helpers whose names still imply authority-layer ownership.

Pros:

- strongest clarity result;
- preserves behavioral safety by separating movement from semantics.

Cons:

- multi-file work;
- docs/test path churn.

## Selected Design

Used **Option 3** and completed both implementation steps.

Final conclusions:

- `test/helpers/reflection-recall-reference.ts` is not a production path and is now explicitly isolated as a test/reference helper.
- `test/helpers/reflection-recall-selection-reference.ts` is not a production path and is now explicitly isolated as a test/reference helper downstream utility.
- `src/prompt-local-auto-recall-selection.ts` is production code and now carries prompt-local naming that matches its actual ownership.
- `src/prompt-local-topk-setwise-selection.ts` remains a shared local utility, but now has prompt-local naming instead of authority-sounding naming.
- `src/recall-engine.ts` and `src/adaptive-retrieval.ts` are real runtime-local helpers and should not be deleted blindly.

## Validation Plan

Audit and post-cleanup evidence commands:

```bash
rg -n "recall-engine|prompt-local-auto-recall-selection|prompt-local-topk-setwise-selection|reflection-recall-reference|reflection-recall-selection-reference|adaptive-retrieval" src test index.ts
npm test
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/ts-residual-debt-cleanup-2026-03-17
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/ts-residual-debt-cleanup-2026-03-17 README.md
```

## Risks and Assumptions

Assumptions:

- current `rg` import evidence is sufficient to distinguish production use from test/reference use for the target files;
- no dynamic import path outside the repo search results is secretly depending on the relocated test helpers under `test/helpers/`.

Risks:

- future production work could still grow new prompt-local helpers with authority-sounding names if the current naming discipline is not preserved;
- the retained `src/recall-engine.ts` name is still broad, but its current imports and behavior make it acceptable for this scope;
- test/reference helpers now live in the correct tree, but maintainers should still avoid importing them into runtime code.
