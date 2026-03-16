## Context

`Chronicle Engine` (formerly `memory-lancedb-pro` in package/repo identity) has gone through multiple architecture resets:

- local-authority removal
- remote-authority consolidation
- Rust backend RAG completion
- deploy layout cleanup

The repository still contains many planning, milestone, contract, and closeout documents that were correct for those transitions but are no longer good "current state" references. Several operator-facing docs also still describe the Rust backend as optional, future, or MVP-only, which is now inaccurate.

## Findings

- `README.md` and `README_CN.md` still point readers at `docs/remote-authority-reset/*` as the canonical active architecture set.
- `deploy/README.md` still describes the Rust backend as a future service, even though `backend/` is present and buildable now.
- `deploy/backend.toml.example` still frames the configuration as a future shape to align later.
- `docs/benchmarking.md` documents removed local CLI / benchmark flows.
- `docs/remote-authority-reset/`, `docs/rust-rag-completion/`, `docs/rust-backend-completion-check/`, `docs/docker-backend-deploy-layout-fix/`, and `docs/archive/` are historical execution artifacts rather than current runtime/operator docs.
- `docs/long-context-chunking.md` documents old plugin-side embedding config knobs instead of the current backend-owned chunk-recovery path.

## Goals / Non-goals

### Goals

- Leave a small, obvious set of current documentation under `docs/`.
- Move outdated or historical documentation under `docs/archive/`.
- Keep historical material intact, but clearly demote it from canonical guidance.
- Update README and deployment docs so they match the current runtime architecture.

### Non-goals

- Rewriting every historical plan into fresh prose.
- Deleting historical execution evidence.
- Changing runtime code or backend behavior.

## Target files / modules

- `README.md`
- `README_CN.md`
- `deploy/README.md`
- `deploy/backend.toml.example`
- `docs/README.md`
- `docs/runtime-architecture.md`
- `docs/long-context-chunking.md`
- `docs/archive-index.md`
- `docs/documentation-refresh/documentation-refresh-contract.md`
- move outdated doc trees into `docs/archive/`

## Constraints

- Preserve historical docs; relocate instead of deleting.
- Keep the active docs set small and easy to scan.
- Prefer current implementation and config files over old plan docs when deciding canonical wording.
- Do not leave README links pointing at moved paths.

## Verification plan

- `git -C /root/.openclaw/workspace/plugins/memory-lancedb-pro status --short`
- `find /root/.openclaw/workspace/plugins/memory-lancedb-pro/docs -maxdepth 2 -type f | sort`
- `rg -n 'docs/(archive|remote-authority-reset|rust-rag-completion|rust-backend-completion-check|docker-backend-deploy-layout-fix)' /root/.openclaw/workspace/plugins/memory-lancedb-pro/README.md /root/.openclaw/workspace/plugins/memory-lancedb-pro/README_CN.md /root/.openclaw/workspace/plugins/memory-lancedb-pro/docs /root/.openclaw/workspace/plugins/memory-lancedb-pro/deploy`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh /root/.openclaw/workspace/plugins/memory-lancedb-pro/docs/documentation-refresh`

## Rollback

- Move directories from `docs/archive/` back to their original `docs/` paths.
- Restore the updated README/deploy/docs files from git if the new current-state wording is judged too aggressive.

## Open questions

- Whether any historical task-plan set should later be collapsed further into a single changelog-style summary.
