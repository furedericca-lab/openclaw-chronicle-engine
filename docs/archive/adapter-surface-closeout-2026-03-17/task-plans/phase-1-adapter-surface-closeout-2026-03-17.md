---
description: Task list for adapter-surface-closeout-2026-03-17 phase 1.
---

# Tasks: adapter-surface-closeout-2026-03-17 Phase 1

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
- Backend remains the only memory/distill/recall authority.
- Ordinary `/v1/recall/*` DTOs must stay narrow; debug trace belongs on explicit debug surfaces only.
- No local reflection-generation fallback may be restored.

## Format
- [ID] [P?] [Component] Description
- [P] means parallelizable.
- Valid components: Backend, Frontend, Agentic, Docs, Config, QA, Security, Infra.
- Every task must have a clear DoD.

## Phase 1: Baseline Freeze
Goal: Convert the scan results into a frozen implementation baseline, contract decision set, and test matrix before touching runtime code.

Definition of Done: All phase tasks are implemented, tested, and evidenced with commands and outputs.

Tasks:
- [x] T001 [Docs] Freeze the exact gap inventory and target file map.
  - DoD: `docs/archive/adapter-surface-closeout-2026-03-17/*.md` identifies the concrete disposition of `src/backend-tools.ts`, `src/backend-client/*`, `index.ts`, `openclaw.plugin.json`, `package.json`, `README.md`, `README_CN.md`, and the directly relevant tests.
- [x] T002 [P] [QA] Freeze the baseline verification matrix for shell, config, and residual-debt changes.
  - DoD: the phase docs reference concrete commands for `npm test`, focused Node test files under `test/`, and `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture` when backend assumptions are touched.
- [x] T003 [Security] Freeze the management-surface and compatibility guardrails.
  - DoD: the docs/contracts explicitly define principal requirements, `enableManagementTools` gating, no-scope-override rules, and the config-compatibility policy for stale `memoryReflection` fields.

Checkpoint: Phase 1 artifacts are updated, verified, and recorded in `4phases-checklist.md` before next phase starts.

## Dependencies & Execution Order
- Phase 1 blocks all others.
- This phase must complete before any later phase starts.
- Tasks marked [P] within this phase may run concurrently only when they do not touch the same files.
