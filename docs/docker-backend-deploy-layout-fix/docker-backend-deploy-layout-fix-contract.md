# docker-backend-deploy-layout-fix Contract

## Context
Master asked for a small repo fix: repair the backend Docker build by updating the Rust toolchain baseline, flatten the nested deploy layout into `deploy/`, and update repo docs/workflow references accordingly.

## Findings
- Current backend Docker assets were stored in a nested deploy subdirectory instead of directly in `deploy/`.
- GitHub workflow `.github/workflows/docker-backend.yml` watched the nested deploy path and built from a nested Dockerfile path.
- The latest Docker Backend workflow failed because the builder image used Rust 1.86 while locked dependencies now require Rust >= 1.88.

## Goals / Non-goals
### Goals
- Move backend deployment artifacts to `deploy/`.
- Update Dockerfile builder Rust version to a dependency-compatible baseline.
- Update workflow/docs/path references so the flattened layout is canonical.
- Keep the fix small, auditable, and mergeable.

### Non-goals
- No feature work in the Rust backend itself.
- No unrelated workflow redesign.
- No image naming changes unless required by the path flattening.

## Target files / modules
- `deploy/Dockerfile` (after move)
- `deploy/README.md`
- `deploy/backend.toml.example`
- `deploy/docker-compose.yml`
- `.github/workflows/docker-backend.yml`
- Any README/docs references to the old nested deploy path

## Constraints
- Preserve current image name and workflow intent.
- Prefer minimal file moves + path updates over broad documentation rewrites.
- Keep persistent docs in English.

## Verification plan
- Inspect changed paths and references with ripgrep.
- Run at least one local backend Docker build or equivalent targeted validation if environment permits.
- Re-run repo doc placeholder/residual scans if docs are updated.

## Rollback
- Revert the path move and Dockerfile baseline change in one commit if build validation or workflow references break.

## Open questions
- Rust builder baseline target resolved to `1.88` (minimum compatible with current lockfile requirements).
- Residual references outside directly affected files will be detected with post-change scans.

## Execution log / evidence updates
- Moved deploy assets from nested subdirectory to:
  - `deploy/Dockerfile`
  - `deploy/README.md`
  - `deploy/backend.toml.example`
  - `deploy/docker-compose.yml`
- Updated `.github/workflows/docker-backend.yml` path filters to `deploy/**` and build file to `deploy/Dockerfile`.
- Updated `deploy/Dockerfile` Rust builder baseline from `1.86` to `1.88`.
- Updated deploy usage docs and compose examples to the flattened deploy layout.
- Verification run (2026-03-16):
  - Legacy-path/toolchain ripgrep residual scan -> no matches (`RG_EXIT_CODE=1`).
  - `doc_placeholder_scan.sh` -> `[OK]`.
  - `post_refactor_text_scan.sh` -> `[OK]`.
  - `docker build -f deploy/Dockerfile .` and one retry with `DOCKER_BUILDKIT=0` both failed on external docker.io connectivity (`connection reset by peer`), not on Dockerfile path/toolchain parsing.
