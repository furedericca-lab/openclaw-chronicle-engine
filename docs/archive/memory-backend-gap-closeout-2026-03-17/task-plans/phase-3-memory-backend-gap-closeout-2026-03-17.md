---
description: Task list for memory-backend-gap-closeout-2026-03-17 phase 3.
---

# Tasks: memory-backend-gap-closeout-2026-03-17 Phase 3

## Input
- Canonical sources:
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/memory-backend-gap-closeout-2026-03-17/memory-backend-gap-closeout-2026-03-17-scope-milestones.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/memory-backend-gap-closeout-2026-03-17/memory-backend-gap-closeout-2026-03-17-technical-documentation.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/memory-backend-gap-closeout-2026-03-17/memory-backend-gap-closeout-2026-03-17-contracts.md

## Canonical architecture / Key constraints
- prompt-local seams that do not recreate backend authority stay local;
- cleanup must not remove test/reference helpers without import-proof and test updates;
- docs/schema/runtime behavior must converge.

## Phase 3: Compatibility and Residue Closeout
Goal: reduce the remaining misleading local-authority vocabulary and file-placement debt after the runtime path is fixed.

Definition of Done: config/schema/docs/runtime naming all match the supported backend-owned model, with residual exceptions explicitly documented.

Tasks:
- [x] T041 [Config] Tighten legacy config messaging for `sessionMemory.*` and deprecated `memoryReflection.*` compatibility fields.
  - DoD: parser behavior, startup warnings, schema text, and migration tests all align.
- [x] T042 [P] [Docs] Update `README.md`, `README_CN.md`, and runtime docs to reflect the implemented reflection source/status behavior and the exact disposition of compatibility fields.
  - DoD: no doc claims contradict the shipped adapter/runtime surface.
- [x] T043 [P] [Agentic] Remove, relocate, or sharply classify test-only helper residue such as `src/query-expander.ts` and `src/reflection-store.ts`.
  - DoD: import-proof is recorded and production paths do not depend on those helpers.
- [x] T044 [QA] Run full regression and hygiene scans.
  - DoD: selected Node/backend tests plus doc placeholder/refactor scans pass, and remaining risks are recorded in the checklist.

Checkpoint: the repo no longer presents the remaining migration gaps as active supported runtime behavior.

## Evidence

- Compatibility-field warnings and schema text align in `index.ts`, `openclaw.plugin.json`, `README.md`, and `README_CN.md`.
- `src/session-recovery.ts` and its path-based test were deleted because the supported runtime no longer uses plugin-local session-file recovery.
- `src/query-expander.ts` and `src/reflection-store.ts` remain import-proofed as test/reference helpers only.

## Verification Commands

- `rg -n "sessionMemory\\.|memoryReflection\\.|deprecatedIgnoredFields" index.ts openclaw.plugin.json README.md README_CN.md test`
- `rg -n "query-expander\\.ts|reflection-store\\.ts" src test README.md README_CN.md`
- `npm test`
- `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/memory-backend-gap-closeout-2026-03-17`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/memory-backend-gap-closeout-2026-03-17 README.md`

## Dependencies & Execution Order
- Depends on Phases 1-2.
- T041-T043 may overlap only when file ownership is disjoint.
- T044 is the release gate for this scope.
