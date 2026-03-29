# Documentation Index

This directory keeps the current operator/runtime references and two intentionally retained top-level historical design snapshots.

Current documents:

- `backend-admin-plane-2026-03-27/`: active phased scope for adding a bundled admin plane under `/admin` and `/admin/api/*`, wiring live admin auth/logging, and tightening duplicated recall flow while preserving the Rust backend as sole authority.
- `runtime-architecture.md`: canonical runtime split and source-of-truth boundaries.

Historical and superseded material:

- `archive/`: completed plans, architecture transition docs, closeout contracts, and stale operator notes preserved for reference.
- `archive-index.md`: top-level archive map for the preserved historical scopes.
- `archive/backend-env-config-2026-03-29/`: archived single-contract scope for baking a default backend config into the image and supporting `CHRONICLE_*` environment overrides so Docker Compose no longer requires a mounted `backend.toml` by default.
- `archive/ghcr-version-retention-2026-03-27/`: archived phased scope for adding workflow-enforced GHCR cleanup so `chronicle-engine-backend` keeps only the newest 10 package versions after successful non-PR publishes.
- `archive/backend-admin-plane-merge-fix-2026-03-29/`: archived phased repair scope for fixing the merged admin-plane regressions, converging the admin API surface back to the parent contract, and restoring a truthful Settings write path plus the backend verification line.
- `archive/backend-binary-rename-chronicle-engine-rs-2026-03-26/`: archived single-contract scope for renaming the Rust backend crate/lib/bin surface to `chronicle-engine-rs` / `chronicle_engine_rs` while intentionally keeping deploy image naming stable.
- `archive/backend-dependency-upgrades-2026-03-25/`: archived phased dependency-upgrade scope for moving the backend onto newer `lancedb`, `axum`, `rusqlite`, and `toml` lines while explicitly closing on single-compatible `reqwest` and `arrow-*` versions instead of keeping a split dependency graph.
- `archive/backend-release-line-closeout-2026-03-25/`: archived phased release-line scope for fixing backend Clippy/reliability blockers, splitting the backend contract suite into the `contract_semantics` multi-file test target, and recording the verification gate for release-quality hygiene.
- `archive/storage-reflection-field-cleanup-2026-03-25/`: archived single-contract scope for cutting over LanceDB storage internals from legacy reflection-era names to behavioral-facing names and rejecting unsupported legacy schemas instead of auto-migrating them.
- `archive/snapshot-refresh-2026-03-25/`: archived single-contract scope for refreshing the retained top-level architecture snapshots to the 2026-03-25 naming and runtime boundary.
- `archive/backend-public-reflection-surface-removal-2026-03-25/`: archived single-contract scope for removing the remaining public reflection-named backend recall aliases while preserving storage-internal reflection schema.
- `archive/test-cleanup-audit-2026-03-25/`: archived single-contract cleanup scope for removing dead test assets and shrinking overexposed test helper exports.
- `archive/backend-behavioral-boundary-closeout-2026-03-19/`: archived phased closeout scope for deciding and closing the remaining backend `reflection` naming boundary plus internal legacy semantic leftovers.
- `archive/governance-behavioral-closeout-2026-03-19/`: archived phased closeout scope for removing governance/self-improvement legacy surfaces, deleting wrapper shims, and documenting the remaining backend naming boundary.
- `archive/autorecall-governance-unification-2026-03-18/`: archived phased unification scope superseded by the 2026-03-19 closeout.
- `context-engine-split-2026-03-25/`: refreshed 2026-03-25 module-placement snapshot for the context-engine split design set. Any older command-triggered reflection-generation wording inside that snapshot is superseded by `runtime-architecture.md` and `remote-memory-backend-2026-03-25/`.
- `remote-memory-backend-2026-03-25/`: refreshed 2026-03-25 architecture/design snapshot for the remote backend design set.

Selection rule:

- If a document describes a migration plan, phased execution, placeholder gap, MVP target state, or completed cleanup, it belongs in `docs/archive/`.
- If a document describes how the current repo works today, it stays in `docs/`.
- Once a scoped phased closeout lands and no longer represents active execution, move it under `docs/archive/`.
- Exception: `context-engine-split-2026-03-25/` and `remote-memory-backend-2026-03-25/` stay top-level as architecture/design snapshots, but they are not the canonical runtime/source-of-truth docs.
