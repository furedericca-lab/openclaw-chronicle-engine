# Documentation Index

This directory keeps the current operator/runtime references plus two intentionally retained top-level historical architecture snapshots.

Current documents:

- `runtime-architecture.md`: canonical runtime split and source-of-truth boundaries.

Historical and superseded material:

- `archive/`: completed plans, architecture transition docs, closeout contracts, and stale operator notes preserved for reference.
- `archive-index.md`: top-level archive map for the preserved historical scopes.
- `context-engine-split-2026-03-15/`: historical 2026-03-15 architecture-reset snapshot for the context-engine split design set.
- `remote-memory-backend-2026-03-17/`: historical 2026-03-17 refreshed architecture-reset snapshot for the remote backend design set.

Selection rule:

- If a document describes a migration plan, phased execution, placeholder gap, MVP target state, or completed cleanup, it belongs in `docs/archive/`.
- If a document describes how the current repo works today, it stays in `docs/`.
- Exception: `context-engine-split-2026-03-15/` and `remote-memory-backend-2026-03-17/` stay top-level as read-only historical architecture snapshots, but they are not the canonical current-state docs.
