---
description: Freeze the strict parity baseline and classify the real remaining gaps.
---

# Tasks: strict-parity-gap-2026-03-17

## Input

- `docs/archive/strict-parity-gap-2026-03-17/strict-parity-gap-2026-03-17-implementation-research-notes.md`
- `docs/archive/strict-parity-gap-2026-03-17/strict-parity-gap-2026-03-17-scope-milestones.md`
- `docs/archive/rust-rag-completion/rust-rag-parity-gap-priority.md`
- `docs/archive/remote-authority-reset/phase-4-closeout-release-notes.md`
- `backend/src/state.rs`
- `index.ts`
- `src/context/*`

## Canonical architecture / Key constraints

- remote backend remains the only supported authority;
- deleted local-authority files must not come back;
- strict parity means historical TS behavior/evidence, not broader feature creep;
- local prompt-time orchestration may remain local only if ownership is explicit and non-authoritative.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 1: Baseline Freeze

Goal: convert the current audit into a frozen strict-parity gap register with explicit ownership boundaries and acceptable-equivalence rules.

Definition of Done: strict parity criteria, retained-helper ownership, acceptable Rust-native replacements, and representative scenario matrix are documented with concrete file evidence and no ambiguity about what remains open.

Tasks:

- [x] T001 [Docs] Freeze the strict parity definition and acceptance language.
  - DoD: `docs/archive/strict-parity-gap-2026-03-17/strict-parity-gap-2026-03-17-implementation-research-notes.md`, `...-contracts.md`, and `technical-documentation.md` agree on what counts as a strict gap versus a closed item or acceptable Rust-native / remote-native replacement.
- [x] T002 [P] [Docs] Build the retained TS helper ownership matrix.
  - DoD: docs classify `src/recall-engine.ts`, `src/auto-recall-final-selection.ts`, `src/reflection-recall.ts`, `src/final-topk-setwise-selection.ts`, and related `src/context/*` seams as either prompt-local or backend-parity debt.
- [x] T003 [P] [QA] Define representative strict-parity scenarios and expected evidence.
  - DoD: docs enumerate concrete scenarios for duplicate-heavy retrieval, reinforced stale memories, rerank fallback, reflection grouping, and diagnostics/trace visibility, with target tests/modules identified.
- [x] T004 [Security] Freeze trace/admin/debug guardrails before implementation.
  - DoD: docs state authorization, redaction, and DTO non-leakage rules for any new trace surface.

Checkpoint: engineering can begin implementation without re-discovering what strict parity means.

## Dependencies & Execution Order

- Phase 1 blocks all others.
- `T002` and `T003` may run in parallel after `T001` starts, but all Phase 1 outputs must align before Phase 2 starts.

## Execution Record

### Implemented

- froze the acceptable-equivalence rule so historical TS behavior is judged at the capability level rather than literal TS-local implementation shape;
- added a retained helper ownership matrix in `strict-parity-gap-2026-03-17-implementation-research-notes.md`;
- classified `src/auto-recall-final-selection.ts` as the primary backend-parity debt candidate and downgraded the other retained helpers to either acceptable prompt-local orchestration or test/reference status;
- added a representative strict-parity scenario matrix covering duplicate-heavy recall, reinforced stale memories, long-input embedding recovery, rerank fallback, reflection recall/injection, and diagnostics inspection;
- froze trace/admin/debug guardrails in docs/contracts before any Phase 2 implementation work.

### Evidence

- source inspection:
  - `src/context/auto-recall-orchestrator.ts`
  - `src/context/reflection-prompt-planner.ts`
  - `src/context/session-exposure-state.ts`
  - `src/context/prompt-block-renderer.ts`
  - `src/recall-engine.ts`
  - `src/auto-recall-final-selection.ts`
  - `src/final-topk-setwise-selection.ts`
  - `backend/src/state.rs`
- archive evidence:
  - `docs/archive/rust-rag-completion/rust-rag-parity-gap-priority.md`
  - `docs/archive/remote-authority-reset/phase-4-closeout-release-notes.md`

### Phase 1 checkpoint result

- completed: strict parity baseline, retained-helper ownership matrix, representative scenario matrix, and trace/admin/debug guardrails are now frozen for later phases.
