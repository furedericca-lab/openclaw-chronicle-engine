---
description: Phase 3 task list for distill-backend-scope.
---

# Tasks: distill-backend-scope

## Input

- `docs/distill-backend-scope/*`
- `docs/remote-memory-backend-2026-03-17/technical-documentation.md`
- `docs/remote-memory-backend-2026-03-17/remote-memory-backend-contracts.md`
- `docs/remote-memory-backend-2026-03-17/README.md`

## Canonical architecture / Key constraints

- remote backend docs must remain historically accurate as a 2026-03-17 snapshot;
- updates should add the missing distill viewpoint without pretending the capability already shipped;
- no runtime behavior claims may be added without code evidence.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid components: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must have a clear DoD.

## Phase 3: Remote Backend Documentation Alignment

Goal: update the remote backend snapshot so distill is explicitly placed in the architecture story.

Definition of Done: the remote backend docs explain what distill is, which enqueue/status skeleton is now shipped, which executor behavior remains deferred, how it differs from reflection and auto-capture, and why the old sidecar residue is cleanup debt rather than alternate architecture.

Tasks:

- [x] T041 [Docs] Update remote backend technical docs with the missing distill viewpoint.
  - DoD: technical docs explain distill as a future backend-native async transcript capability and mark the current sidecar as non-canonical.
- [x] T042 [P] [Docs] Update remote backend contracts with the intended future contract direction and non-goals.
  - DoD: contracts describe distill as a deferred future surface without implying it already ships.
- [x] T043 [QA] Re-run repo-task-driven scans on the new scope after documentation alignment.
  - DoD: doc placeholder scan and post-refactor text scan pass.
- [x] T044 [Docs] Document the cleanup/disposition intent in the remote backend snapshot.
  - DoD: remote backend docs make it clear that `jsonl_distill.py` and the example distiller are reference debt, not indefinite supported runtime architecture.

Checkpoint: the repo has one coherent explanation for the current sidecar distiller residue and its future backend-native direction.
