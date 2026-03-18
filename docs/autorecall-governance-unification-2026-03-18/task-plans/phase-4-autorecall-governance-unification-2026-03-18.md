---
description: Task list for autorecall-governance-unification-2026-03-18 phase 4.
---

# Tasks: autorecall-governance-unification-2026-03-18 Phase 4

## Input
- Canonical sources:
  - /root/verify/openclaw-chronicle-engine-autorecall-governance-unification-2026-03-18/README.md
  - /root/verify/openclaw-chronicle-engine-autorecall-governance-unification-2026-03-18/docs/autorecall-governance-unification-2026-03-18/autorecall-governance-unification-2026-03-18-scope-milestones.md
  - /root/verify/openclaw-chronicle-engine-autorecall-governance-unification-2026-03-18/docs/autorecall-governance-unification-2026-03-18/autorecall-governance-unification-2026-03-18-technical-documentation.md
  - /root/verify/openclaw-chronicle-engine-autorecall-governance-unification-2026-03-18/docs/autorecall-governance-unification-2026-03-18/autorecall-governance-unification-2026-03-18-contracts.md

## Canonical architecture / Key constraints
- Keep architecture aligned with autorecall-governance-unification-2026-03-18 scope docs and contracts.
- Keep provider/runtime/channel boundaries unchanged unless explicitly in scope.
- Keep security and test gates in Definition of Done.

## Format
- [ID] [P?] [Component] Description
- [P] means parallelizable.
- Valid components: Backend, Frontend, Agentic, Docs, Config, QA, Security, Infra.
- Every task must have a clear DoD.

## Phase 4: Verification And Closeout
Goal: Prove the refactor, close the scope docs, and record remaining compatibility notes.

Definition of Done: All phase tasks are implemented, tested, and evidenced with commands and outputs.

Tasks:
- [ ] T061 [QA] Run targeted JS and Rust verification for changed surfaces.
  - DoD: targeted `node --test ...` commands and `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture` are executed and outcomes are recorded in the checklist.
- [ ] T062 [P] [Docs] Run required doc hygiene scans and remove residual placeholders.
  - DoD: `doc_placeholder_scan.sh` and `post_refactor_text_scan.sh` pass for `docs/autorecall-governance-unification-2026-03-18` and `README.md`; failures, if any, are fixed and re-run.
- [ ] T063 [Security] Record remaining compatibility/risk notes in scope docs.
  - DoD: the checklist and technical docs summarize changed files, verification commands, remaining gaps, and every transitional alias retained by the implementation.

Checkpoint: Phase 4 artifacts are merged, verified, and recorded in 4phases-checklist.md before next phase starts.

## Dependencies & Execution Order
- Phase 1 blocks all others.
- Phase 4 depends on completion of Phases 1-3.
- Tasks marked [P] within this phase may run concurrently only when they do not touch the same files.
