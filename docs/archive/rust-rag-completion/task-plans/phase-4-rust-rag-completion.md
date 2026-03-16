---
description: Verification and gap-closeout tasks for rust-rag-completion.
---

# Tasks: rust-rag-completion Phase 4

## Input
- Canonical sources:
  - /root/verify/memory-lancedb-pro-context-engine-split/README.md
  - /root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/rust-rag-completion-scope-milestones.md
  - /root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/technical-documentation.md
  - /root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/rust-rag-completion-contracts.md
  - /root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/task-plans/4phases-checklist.md

## Canonical architecture / Key constraints
- Keep Rust backend as the only runtime RAG authority.
- Preserve stable `/v1` DTO boundaries.
- Prioritize production-safety gaps before parity polish.
- Do not restore deleted local TS authority paths.

## Format
- [ID] [P?] [Component] Description
- [P] means parallelizable.
- Valid components: Backend, Frontend, Agentic, Docs, Config, QA, Security, Infra.
- Every task must have a clear DoD.

## Phase 4: Gap closeout toward TS parity
Goal: close the highest-value residual gaps after the provider-based Rust RAG main path landed.

Definition of Done: schema compatibility, index lifecycle, and selected high-value parity gaps are implemented or explicitly deferred with evidence.

Tasks:
- [ ] T401 [Backend] Implement compatibility handling for pre-existing Lance tables without the `vector` column.
  - DoD: backend can detect and safely handle or migrate old tables; behavior is verified by test or explicit compatibility check.
- [ ] T402 [Backend] Add explicit vector-index lifecycle management for vector retrieval paths.
  - DoD: vector index creation/ensure logic exists where appropriate and verification proves it is not implicit handwaving.
- [ ] T403 [P] [Backend] Port the highest-value missing TS retrieval-quality features.
  - DoD: at least one of query expansion, noise filtering, safe embedding-cache reuse, or similarly material retrieval-quality behavior lands with focused tests.
- [ ] T404 [P] [QA] Add regression tests for the newly closed gaps.
  - DoD: targeted tests cover old-table compatibility, index lifecycle expectations, and any newly landed retrieval-quality behavior.
- [ ] T405 [Docs] Update `docs/rust-rag-completion/task-plans/4phases-checklist.md` with exact commands, outcomes, and remaining deferred items.
  - DoD: checklist clearly separates closed gaps from still-deferred parity items.

Checkpoint: Phase 4 closes the top production-risk gaps and records explicit residuals before merge/acceptance.

## Dependencies & Execution Order
- T401 and T402 should be resolved before claiming production-ready parity.
- T403 may proceed after the migration/index plan is concrete.
- T404/T405 close the phase.
