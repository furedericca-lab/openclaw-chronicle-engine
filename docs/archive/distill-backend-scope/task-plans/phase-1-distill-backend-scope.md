---
description: Phase 1 task list for distill-backend-scope.
---

# Tasks: distill-backend-scope

## Input

- `scripts/jsonl_distill.py`
- `examples/new-session-distill/*`
- `docs/remote-memory-backend-2026-03-17/remote-memory-backend-2026-03-17-technical-documentation.md`
- `docs/remote-memory-backend-2026-03-17/remote-memory-backend-contracts.md`

## Canonical architecture / Key constraints

- treat reflection and auto-capture as already-shipped backend capabilities;
- do not treat the sidecar distiller as canonical runtime architecture;
- classify residue before planning any future backend-native contract.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid components: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must have a clear DoD.

## Phase 1: Distill Residue Audit

Goal: produce a file-by-file classification and capability map for current distill residue.

Definition of Done: current distiller residue is classified, reusable techniques are separated from sidecar-only deployment details, each residue item has an initial cleanup class, and the current sidecar test surface is mapped to future backend tests.

Tasks:

- [x] T001 [Docs] Audit current distiller residue under `scripts/`, `examples/`, and tests.
  - DoD: docs record current role and status for `jsonl_distill.py`, example hook/worker artifacts, and related tests.
- [x] T002 [P] [Docs] Freeze the capability boundary between reflection, auto-capture, and distill.
  - DoD: implementation notes and technical docs explain the three capabilities without overlap confusion.
- [x] T003 [Security] Record the authority rule that sidecar import/persistence is not a valid future authority model.
  - DoD: contracts explicitly reject `memory-pro import` style sidecar persistence as canonical architecture.
- [x] T004 [Docs] Freeze the initial cleanup class for each residue item.
  - DoD: implementation notes record which files are temporary migration reference, example drift debt, or future archive/remove targets.
- [x] T005 [QA] Freeze a future backend test matrix for `jsonl_distill.py` behavior.
  - DoD: technical docs and research notes map current sidecar filtering/cursor behaviors to future backend test classes.

Checkpoint: the repo has a concrete baseline for distill planning without rediscovering what the existing residue does.
