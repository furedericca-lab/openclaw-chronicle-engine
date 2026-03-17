---
description: Phase 2 task list for distill-backend-scope.
---

# Tasks: distill-backend-scope

## Input

- `docs/distill-backend-scope/distill-backend-scope-implementation-research-notes.md`
- `docs/archive/distill-backend-scope/distill-backend-scope-technical-documentation.md`
- `docs/distill-backend-scope/distill-backend-scope-contracts.md`

## Canonical architecture / Key constraints

- the future target is backend-native distill jobs, not sidecar preservation;
- reflection and auto-capture remain separate capabilities;
- this phase freezes the design baseline that later runtime code must follow without reopening the contract.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid components: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must have a clear DoD.

## Phase 2: Backend-Native Distill Design

Goal: define what should migrate into a future Rust backend distill capability.

Definition of Done: the docs freeze a backend-native target design, a capability absorption map, a code-ready DTO/job-state contract direction, and a long-term cleanup path for old residue.

Tasks:

- [x] T021 [Backend] Define the future backend-native distill pipeline stages.
  - DoD: technical docs specify ingest, cleaning, chunking, extraction, reduce, and persistence stages.
- [x] T022 [P] [Docs] Freeze the absorption map from current sidecar residue to future backend-native capability.
  - DoD: research notes classify what should be absorbed, rejected, or left as example-only.
- [x] T023 [Security] Freeze ownership and authority rules for future distill jobs.
  - DoD: contracts state that future distill jobs must be backend-owned, caller-scoped, and not use sidecar import persistence.
- [x] T024 [Docs] Freeze the planned cleanup map after backend-native parity.
  - DoD: docs specify which residue will be archived, removed, or replaced by backend-native tests/implementation.
- [x] T025 [Backend] Freeze initial DTO and job-state model to implementation-prep depth.
  - DoD: contracts/technical docs define initial request fields, response fields, enums, job states, and table/storage direction closely enough that implementation can start without reopening those decisions.

Checkpoint: future backend-native distill work can start from a concrete architecture and contract baseline.
