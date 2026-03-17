# remote-authority-reset Docs Index

Canonical documents (active):

- `remote-authority-reset-contracts.md`
- `remote-authority-reset-technical-documentation.md`
- `remote-authority-reset-scope-milestones.md`
- `remote-authority-reset-implementation-research-notes.md`
- `remote-authority-reset-brainstorming.md`
- `remote-only-local-authority-removal-plan.md`
- `phase-4-closeout-release-notes.md`
- `task-plans/4phases-checklist.md`
- `task-plans/phase-1-remote-authority-reset.md`
- `task-plans/phase-2-remote-authority-reset.md`
- `task-plans/phase-3-remote-authority-reset.md`
- `task-plans/phase-4-remote-authority-reset.md`

Archive:

- `../archive/2026-03-15-architecture-reset/context-engine-split/`
- `../archive/2026-03-15-architecture-reset/remote-memory-backend/`

Purpose:

- Keep one canonical architecture target:
  - Rust remote backend as the only memory/RAG authority.
  - Thin OpenClaw adapter for runtime integration.
  - Local context-engine for prompt-time orchestration only.

Execution order for implementation:

1. Read `remote-authority-reset-contracts.md` and `remote-authority-reset-technical-documentation.md`.
2. Execute staged deletions from `remote-only-local-authority-removal-plan.md`.
3. Follow `task-plans/phase-2-remote-authority-reset.md` through `phase-4-remote-authority-reset.md`.
4. Record evidence in `task-plans/4phases-checklist.md`.

Current state:

- Phase 2 hard remote-only runtime/schema enforcement is complete.
- Phase 3 hard deletion is complete: remaining local-authority runtime modules/tests were removed and permanent modules were type-uncoupled from local store/retriever/embedder files.
- Phase 4 closeout is complete: regression/doc hygiene evidence, release-cut notes, rollback discipline, and checklist closeout are recorded.
