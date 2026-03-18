---
description: Scope boundaries and milestones for autorecall-governance-unification-2026-03-18.
---

# autorecall-governance-unification-2026-03-18 Scope and Milestones

## In Scope
- `index.ts` runtime wiring and config parsing for the new autoRecall/governance naming model.
- `src/context/*` prompt-time orchestration changes that absorb former reflection recall/injection into autoRecall behavioral guidance.
- Governance tool/module/file migration away from `self_improvement_*`.
- `src/backend-tools.ts` and `src/backend-client/types.ts` renames or terminology demotion needed to keep public debug/tooling surfaces coherent.
- Active docs and tests:
  - `README.md`
  - `README_CN.md`
  - `docs/runtime-architecture.md`
  - `docs/context-engine-split-2026-03-17/*` where current wording would otherwise mislead
  - targeted `test/` files and `backend/tests/phase2_contract_semantics.rs` verification if touched assumptions need confirmation
- Scope docs and phased closeout artifacts under `docs/autorecall-governance-unification-2026-03-18/`.

## Out of Scope
- Renaming Rust backend routes, persisted DB columns, or LanceDB reflection metadata in this scope.
- Changing distill artifact semantics beyond the already established boundary that distill owns trajectory-derived generation.
- New external services, new storage backends, or non-local governance workflow systems.

## Milestones
| Milestone | Status | Notes |
|---|---|---|
| 1 | Completed | Discovery docs, compatibility policy, and phase plans were written under `docs/autorecall-governance-unification-2026-03-18/`. |
| 2 | Completed | Runtime/config now present autoRecall as the canonical prompt-time orchestration surface. |
| 3 | Completed | Governance naming owns backlog workflow tools/files and active docs/tests were updated. |
| 4 | Completed | Targeted JS tests, Rust tests, and doc scans all passed on 2026-03-18. |

### Milestone 1: Discovery and contract freeze
- Acceptance gate:
  - scope docs contain concrete baseline evidence, selected design, compatibility policy, and executable phase tasks.

### Milestone 2: Runtime orchestration unification
- Acceptance gate:
  - active plugin runtime no longer wires a separate reflection planner as a peer concept.
  - behavioral recall/error guidance is served through the autoRecall architecture.
  - config parser accepts the canonical names and rejects removed legacy config aliases.

### Milestone 3: Governance surface migration
- Acceptance gate:
  - active tool/module/docs/test surfaces use governance naming instead of self-improvement naming.
  - reminder/bootstrap behavior is described and wired as behavioral guidance rather than governance workflow.

### Milestone 4: Verification and closeout
- Acceptance gate:
  - targeted JS tests pass for changed plugin/runtime/tooling surfaces.
  - targeted Rust tests pass for the changed or revalidated backend-facing assumptions.
  - placeholder scan and post-refactor residual scan pass on the scope docs.
  - closeout notes record changed files, verification commands, remaining gaps, and remaining intentional tool/module aliases.

## Dependencies
- Milestone 2 depends on Milestone 1 because the config/runtime naming split needs a documented contract before refactor.
- Milestone 3 depends on Milestone 2 because governance naming must match the new runtime/config architecture.
- Milestone 4 depends on Milestones 1-3 because tests/docs/scan outputs are the release gate evidence.

## Exit Criteria
- Active runtime/docs/config/tests present autoRecall as the only prompt-time recall/injection orchestration surface.
- Governance clearly owns backlog/review/extraction/promotion workflows and files.
- Self-improvement naming is removed from active canonical config surfaces; only the retained tool/module shims stay as explicit compatibility aliases.
- Reflection-generation-era naming is removed or demoted wherever practical in active code/docs/tests/config.
- Distill remains the only generation authority for trajectory-derived outputs.
- Status: achieved in this verify worktree on 2026-03-18, pending user review/merge only.
