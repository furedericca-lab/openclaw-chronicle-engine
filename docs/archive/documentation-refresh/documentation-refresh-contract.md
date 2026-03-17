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

- `git -C /root/.openclaw/workspace/plugins/openclaw-chronicle-engine status --short`
- `find /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs -maxdepth 2 -type f | sort`
- `rg -n 'docs/(archive|remote-authority-reset|rust-rag-completion|rust-backend-completion-check|docker-backend-deploy-layout-fix)' /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/README.md /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/README_CN.md /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/deploy`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/documentation-refresh`

## Rollback

- Move directories from `docs/archive/` back to their original `docs/` paths.
- Restore the updated README/deploy/docs files from git if the new current-state wording is judged too aggressive.

## Open questions

- Whether any historical task-plan set should later be collapsed further into a single changelog-style summary.

## Implementation updates

- Updated active reader-facing docs so the canonical current-state set is now:
  - `README.md`
  - `README_CN.md`
  - `docs/README.md`
  - `docs/runtime-architecture.md`
  - `deploy/README.md`
- Moved execution-history material out of the active guidance path and surfaced it via `docs/archive/` plus `docs/archive-index.md`.
- Kept the architecture-reset snapshots available as historical references under:
  - `docs/context-engine-split-2026-03-15/`
  - `docs/remote-memory-backend-2026-03-17/`
- Removed the stale active-doc references called out in the original findings:
  - `README.md` and `README_CN.md` no longer point to `docs/remote-authority-reset/*` as canonical guidance.
  - `deploy/README.md` now describes the Rust backend as the current supported backend service rather than future work.
  - top-level `docs/README.md` now states that migration plans, phased docs, and closeout docs belong in archive form.

## Closeout status

Status: complete

Resolved against the original goals:

- A small active docs set exists under `docs/`.
- Historical material remains preserved and demoted from canonical guidance.
- README and deploy docs now align with the remote-authority runtime architecture.

Not closed by this scope:

- Historical archive content still contains old path references, task-plan wording, and execution-time assumptions by design; that material remains preserved as archive evidence, not current guidance.
- A future archive-condensation pass may still be worthwhile if the historical sets become too noisy for operators.

## Verification results

Executed on the current repo/worktree:

- `git -C /root/.openclaw/workspace/plugins/openclaw-chronicle-engine status --short`
  - no outstanding diff was required for the active documentation set before this closeout update.
- `find /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs -maxdepth 2 -type f | sort`
  - confirmed the active top-level docs set is narrow and that historical scopes live under `docs/archive/` or dated snapshot directories.
- `rg -n 'docs/(archive|remote-authority-reset|rust-rag-completion|rust-backend-completion-check|docker-backend-deploy-layout-fix)' ...`
  - active README/deploy/docs surfaces no longer point readers at `docs/remote-authority-reset/*` as canonical documentation.
  - remaining matches are expected inside archive/history material and this contract itself.
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/documentation-refresh`
  - pass: `[OK] placeholder scan clean`

## Residual risks

- This contract originally captured pre-refresh findings and therefore remains partly historical even after closeout; future readers should treat the `Findings` section as the starting state that was resolved by this scope.
- Archive sets still include legacy repo names and old absolute paths in preserved task artifacts; that is acceptable for audit history, but those files should not be promoted back into active guidance without a cleanup pass.
