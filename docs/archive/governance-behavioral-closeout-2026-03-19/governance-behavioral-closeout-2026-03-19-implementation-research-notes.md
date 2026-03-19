---
description: Implementation research notes for governance-behavioral-closeout-2026-03-19.
---

# governance-behavioral-closeout-2026-03-19 Implementation Research Notes

## Baseline (Current State)

- Governance runtime still had legacy compatibility surfaces before this closeout:
  - `src/governance-tools.ts` registered `self_improvement_*` aliases behind `registerLegacyAliases`.
  - `src/governance-tools.ts` still copied backlog files from legacy `.learnings/`.
  - `src/self-improvement-tools.ts` existed only as a compatibility re-export wrapper.
- Prompt-time behavioral guidance still had reflection-named wrappers:
  - `src/context/reflection-prompt-planner.ts`
  - `src/context/reflection-error-signals.ts`
- `index.ts` still normalized hidden reflection-era config aliases that were no longer documented by the schema:
  - `autoRecallExcludeReflection`
  - `inheritance-only` / `inheritance+derived`
  - `includeKinds=["invariant" | "derived"]`
- Adapter/client and some user-facing management/debug surfaces still exposed reflection wording:
  - `src/backend-client/types.ts`
  - `src/backend-client/client.ts`
  - `src/backend-tools.ts`
- `README.md` and `README_CN.md` still advertised transitional `self_improvement_*` tool ids.
- The previous unification scope still lived at `docs/autorecall-governance-unification-2026-03-18/` instead of `docs/archive/`.

## Gap Analysis

- The repo had already selected governance and behavioral-guidance semantics, but the remaining alias/shim surfaces meant the codebase still described two competing names for the same workflow.
- Hidden config alias parsing was riskier than explicit rejection because `openclaw.plugin.json` had already converged to canonical fields.
- The backend contract could not be safely renamed in one closeout without touching stored category semantics, debug routes, and archived design references.
- The docs index/archive layout did not reflect that the previous unification scope was already superseded by this closeout.

## Candidate Designs and Trade-offs

- Full backend wire/storage rename:
  - cleanest terminology outcome;
  - too much churn for a closeout scope.
- Docs-only cleanup:
  - low effort;
  - leaves dead code and alias registration in place.
- Canonical surface cleanup plus documented backend boundary:
  - removes active confusion;
  - preserves route/storage correctness.

## Selected Design

- Canonicalize all active plugin/runtime governance surfaces.
- Delete dead shim modules instead of keeping compatibility wrappers.
- Remove hidden runtime alias parsing and reject those inputs explicitly.
- Rename adapter/client/runtime and backend helper names toward behavioral-guidance wording while keeping the backend wire/storage contract stable.
- Archive the old unification scope and update docs indexes accordingly.

## Validation Plan

- JS runtime/tool regression coverage:
  - `npm test`
- Backend verification because backend helper/handler names changed:
  - `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
- Doc hygiene:
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/governance-behavioral-closeout-2026-03-19`
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/governance-behavioral-closeout-2026-03-19 README.md README_CN.md docs/runtime-architecture.md docs/README.md docs/archive-index.md`

## Risks and Assumptions

- Assumption: removing `.learnings/` read-through is acceptable for this pre-release closeout because no migration note is requested and the repo should stop carrying legacy helper surfaces.
- Assumption: top-level historical design snapshots outside this scope may still contain older wording; the docs index/runtime architecture now carries the source-of-truth clarification for current semantics.
- Remaining intentional boundary:
  - backend routes and persisted row/category semantics still use `reflection`;
  - active adapter/runtime/tool/docs wording is behavioral guidance.
