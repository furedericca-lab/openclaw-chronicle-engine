---
description: Task list for memory-v1-beta-cutover-2026-03-17 phase 3.
---

# Tasks: memory-v1-beta-cutover-2026-03-17 Phase 3

## Input
- Canonical sources:
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/test/helpers/query-expander-reference.ts
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/test/helpers/reflection-store-reference.ts
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/test/query-expander.test.mjs
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/test/memory-reflection.test.mjs
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/src/self-improvement-tools.ts
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/test/self-improvement.test.mjs

## Canonical architecture / Key constraints
- helper cleanup must not break test reference behavior;
- runtime-owned `src/` modules should not contain obvious test-only authority artifacts if avoidable;
- shipped user-facing strings should not contain placeholder checklist markers.

## Phase 3: Residue Cleanup
Goal: remove the remaining repo-layout and shipped-text debt.

Definition of Done: helper ownership is no longer misleading and self-improvement output has no placeholder checklist text.

Tasks:
- [x] T041 [Agentic] Relocate or rename `src/query-expander.ts` and `src/reflection-store.ts` so their ownership is explicitly test/reference-only.
  - DoD: supported runtime imports remain absent; tests import the new canonical location/name.
- [x] T042 [P] [Agentic] Remove placeholder `[TODO]` output from `src/self-improvement-tools.ts`.
  - DoD: user-facing output is concrete or neutral, and `test/self-improvement.test.mjs` is updated accordingly.
- [x] T043 [P] [QA] Update README classification text and import-proof scans for the helper disposition.
  - DoD: active docs and grep evidence match the actual file layout after cleanup.

Checkpoint: repository layout and shipped output no longer advertise avoidable debt.

## Evidence

- `src/query-expander.ts` and `src/reflection-store.ts` moved to `test/helpers/query-expander-reference.ts` and `test/helpers/reflection-store-reference.ts`.
- Tests now import the helper references from `test/helpers/`.
- `src/self-improvement-tools.ts` no longer emits placeholder checklist markers.

## Verification Commands

- `npm test`
- `rg -n "src/query-expander\\.ts|src/reflection-store\\.ts|\\[TODO\\]" package.json package-lock.json openclaw.plugin.json README.md README_CN.md index.ts test src`

## Dependencies & Execution Order
- Depends on Phases 1-2.
- T041-T043 may overlap only when file ownership is disjoint.
