---
description: Task list for governance-behavioral-closeout-2026-03-19 phase 2.
---

# Tasks: governance-behavioral-closeout-2026-03-19 Phase 2

## Input
- Canonical sources:
  - /root/verify/openclaw-chronicle-engine-governance-behavioral-closeout-2026-03-19/README.md
  - /root/verify/openclaw-chronicle-engine-governance-behavioral-closeout-2026-03-19/docs/archive/governance-behavioral-closeout-2026-03-19/governance-behavioral-closeout-2026-03-19-scope-milestones.md
  - /root/verify/openclaw-chronicle-engine-governance-behavioral-closeout-2026-03-19/docs/archive/governance-behavioral-closeout-2026-03-19/governance-behavioral-closeout-2026-03-19-technical-documentation.md
  - /root/verify/openclaw-chronicle-engine-governance-behavioral-closeout-2026-03-19/docs/archive/governance-behavioral-closeout-2026-03-19/governance-behavioral-closeout-2026-03-19-contracts.md

## Phase 2: Canonical Governance Surface

Goal: remove the legacy governance/self-improvement runtime surfaces and make governance the only active workflow/tooling name.

Definition of Done:
- legacy governance tool aliases are gone;
- governance backlog initialization is `.governance` only;
- wrapper modules are deleted and references updated;
- README files stop advertising legacy aliases.

Tasks:
- [x] T021 [Agentic] Remove governance alias/shim runtime code.
  - DoD: `src/governance-tools.ts` registers only canonical governance tools; `src/self-improvement-tools.ts` is deleted.
- [x] T022 [QA] Update tests for canonical governance-only behavior.
  - DoD: `test/governance-tools.test.mjs` checks canonical tool names only.
- [x] T023 [Docs] Remove public legacy governance alias wording.
  - DoD: `README.md` and `README_CN.md` no longer list `self_improvement_*` tools.

Evidence commands:
- `rg -n "self_improvement_|registerLegacyAliases|\\.learnings" src README.md README_CN.md test --glob '!docs/archive/**'`

Checkpoint:
- Phase 2 complete. Governance is the only active backlog/review workflow surface.
