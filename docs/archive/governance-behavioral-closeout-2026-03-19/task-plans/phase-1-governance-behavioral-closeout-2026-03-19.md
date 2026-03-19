---
description: Task list for governance-behavioral-closeout-2026-03-19 phase 1.
---

# Tasks: governance-behavioral-closeout-2026-03-19 Phase 1

## Input
- Canonical sources:
  - /root/verify/openclaw-chronicle-engine-governance-behavioral-closeout-2026-03-19/README.md
  - /root/verify/openclaw-chronicle-engine-governance-behavioral-closeout-2026-03-19/docs/archive/governance-behavioral-closeout-2026-03-19/governance-behavioral-closeout-2026-03-19-scope-milestones.md
  - /root/verify/openclaw-chronicle-engine-governance-behavioral-closeout-2026-03-19/docs/archive/governance-behavioral-closeout-2026-03-19/governance-behavioral-closeout-2026-03-19-technical-documentation.md
  - /root/verify/openclaw-chronicle-engine-governance-behavioral-closeout-2026-03-19/docs/archive/governance-behavioral-closeout-2026-03-19/governance-behavioral-closeout-2026-03-19-contracts.md

## Phase 1: Contract Freeze and Baseline Audit

Goal: replace the scaffold with a repo-accurate execution contract and lock the naming boundary before implementation.

Definition of Done:
- scope docs describe the real repo state and touched modules;
- the chosen rename/archive boundary is explicit;
- verification commands are defined up front.

Tasks:
- [x] T001 [Docs] Record the baseline legacy surfaces.
  - DoD: docs capture the active alias/shim/config/doc/archive residue in `src/governance-tools.ts`, `src/self-improvement-tools.ts`, `src/context/*`, `index.ts`, README files, and the previous scope docs.
- [x] T002 [Docs] Choose the safe backend boundary.
  - DoD: docs state that plugin/runtime-facing naming becomes canonical while backend wire/storage `reflection` naming remains intentionally stable in this scope.
- [x] T003 [QA] Define verification gates.
  - DoD: `npm test`, backend cargo verification, placeholder scan, and post-refactor text scan are recorded here and in the checklist.

Evidence commands:
- `rg -n "self_improvement_|registerLegacyAliases|autoRecallExcludeReflection|inheritance-only|inheritance\\+derived|reflection-prompt-planner|reflection-error-signals" src index.ts README.md README_CN.md`
- `rg -n "governance-behavioral-closeout-2026-03-19" docs`

Checkpoint:
- Phase 1 complete. Scope docs now describe the real closeout instead of scaffold placeholders.
