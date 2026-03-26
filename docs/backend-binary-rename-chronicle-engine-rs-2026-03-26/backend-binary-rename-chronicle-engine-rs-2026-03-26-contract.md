---
description: Single-contract scope for renaming the Rust backend crate and binary to chronicle-engine-rs.
---

# backend-binary-rename-chronicle-engine-rs-2026-03-26 Contract

## Context
- The repo, deploy surface, and plugin branding have already moved to `chronicle-engine`.
- The Rust backend crate and binary still expose the older `memory-lancedb-pro-backend` name in `backend/Cargo.toml`, `backend/src/main.rs`, `deploy/Dockerfile`, `deploy/README.md`, and backend contract-test fixture names.

## Findings
- The active backend crate/package/bin names are still:
  - `memory-lancedb-pro-backend`
  - `memory_lancedb_pro_backend`
- The Docker build and runtime entrypoint still compile/copy/exec the old binary name.
- Active deploy docs still describe the release binary with the old backend name.

## Goals / Non-goals
- Goals:
  - Rename the Rust package, library crate, and binary to `chronicle-engine-rs` / `chronicle_engine_rs`.
  - Update backend source imports, Docker build/runtime entrypoints, deploy docs, and active backend contract-test naming residue.
  - Keep current runtime behavior, API routes, storage schema, and deploy directory layout unchanged.
- Non-goals:
  - Renaming the container image (`chronicle-engine-backend`) or deploy directory paths.
  - Rewriting archive docs that intentionally preserve historical names.
  - Changing public HTTP contracts or plugin ids.

## Target files / modules
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/Cargo.toml`
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/src/main.rs`
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/tests/contract_semantics.rs`
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/tests/contract_semantics/provider_and_retrieval.rs`
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/tests/contract_semantics/diagnostics_auth_and_persistence.rs`
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/tests/contract_semantics/distill_contracts.rs`
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/deploy/Dockerfile`
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/deploy/README.md`
- `/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/Cargo.lock`

## Constraints
- Keep a single consistent `chronicle-engine-rs` backend binary name across build and runtime entrypoints.
- Do not change deploy image naming (`chronicle-engine-backend`) in this scope.
- Preserve backend test coverage and release-line verification gates.

## Verification plan
- `cargo clippy --manifest-path backend/Cargo.toml --all-targets --all-features -- -D warnings`
- `cargo test --manifest-path backend/Cargo.toml --test contract_semantics -- --nocapture`
- `cargo build --manifest-path backend/Cargo.toml --locked --release --bin chronicle-engine-rs`
- `git diff --check`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/backend-binary-rename-chronicle-engine-rs-2026-03-26`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/backend-binary-rename-chronicle-engine-rs-2026-03-26 README.md`

## Rollback
- Revert the Cargo package/lib/bin names, backend source imports, Dockerfile binary references, and deploy doc wording to the pre-scope names.
- Rebuild the backend release binary and rerun contract tests.

## Open questions
- None. This scope keeps deploy path/image naming stable and only renames the Rust backend package/lib/bin surface.
