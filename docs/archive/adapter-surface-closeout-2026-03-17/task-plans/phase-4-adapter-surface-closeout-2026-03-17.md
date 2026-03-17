---
description: Task list for adapter-surface-closeout-2026-03-17 phase 4.
---

# Tasks: adapter-surface-closeout-2026-03-17 Phase 4

## Input
- Canonical sources:
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/README.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/adapter-surface-closeout-2026-03-17/adapter-surface-closeout-2026-03-17-scope-milestones.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/adapter-surface-closeout-2026-03-17/adapter-surface-closeout-2026-03-17-technical-documentation.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/adapter-surface-closeout-2026-03-17/adapter-surface-closeout-2026-03-17-contracts.md

## Canonical architecture / Key constraints
- Keep architecture aligned with adapter-surface-closeout-2026-03-17 scope docs and contracts.
- Keep provider/runtime/channel boundaries unchanged unless explicitly in scope.
- Keep security and test gates in Definition of Done.
- Residual cleanup must be backed by import/use-site proof, not assumption.
- Final docs/package state must match the shipped remote-authority runtime.

## Format
- [ID] [P?] [Component] Description
- [P] means parallelizable.
- Valid components: Backend, Frontend, Agentic, Docs, Config, QA, Security, Infra.
- Every task must have a clear DoD.

## Phase 4: Residual Debt Closeout
Goal: Remove or relocate final misleading artifacts and align package/docs/test surfaces with the finished implementation.

Definition of Done: All phase tasks are implemented, tested, and evidenced with commands and outputs.

Tasks:
- [x] T061 [Config] Remove or relocate residual unused local RAG/package artifacts with import-proof.
  - DoD: `package.json`, `package-lock.json`, and any targeted TS helper paths are updated only after use-site verification proves they are not needed by the active runtime or retained test/reference fixtures.
- [x] T062 [P] [Docs] Align README, README_CN, schema descriptions, and archive references with the shipped surface.
  - DoD: the docs no longer overclaim distill/debug/setwise/local-reflection behavior, and retained local seams are described precisely.
- [x] T063 [P] [QA] Run full regression and refactor hygiene scans.
  - DoD: `npm test`, focused test commands as needed, `doc_placeholder_scan.sh`, and `post_refactor_text_scan.sh` pass; any backend-facing changes also record `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`.
- [x] T064 [Security] Record final residual risks and release guardrails.
  - DoD: `task-plans/4phases-checklist.md` records the final gating state, remaining follow-up items, and confirms that no new management surface bypasses existing auth or caller scoping.

Checkpoint: Phase 4 artifacts are updated, verified, and recorded in `4phases-checklist.md` before next phase starts.

## Dependencies & Execution Order
- Phase 1 blocks all others.
- Phase 4 depends on completion of phases 1-3.
- Tasks marked [P] within this phase may run concurrently only when they do not touch the same files.
