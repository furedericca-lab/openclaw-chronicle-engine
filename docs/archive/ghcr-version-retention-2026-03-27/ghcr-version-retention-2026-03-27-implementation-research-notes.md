description: Implementation notes for GHCR backend image retention enforcement.
---

# ghcr-version-retention-2026-03-27 Implementation Research Notes

## Baseline (Current State)

- `.github/workflows/docker-backend.yml` builds and pushes `ghcr.io/<owner>/chronicle-engine-backend`.
- The workflow had no automated cleanup step for old GHCR package versions.
- The local shell token can push repo changes but lacks `read:packages`, so it cannot audit the current package version count directly.

## Gap Analysis

- Without retention enforcement, the container package can accumulate old versions indefinitely.
- Operators reading only the deploy workflow had no documented retention expectation.

## Candidate Designs and Trade-offs

- `actions/github-script` with GitHub REST package APIs:
  - Pros: no extra script file, direct package version list/delete calls, easy owner-type branching.
  - Cons: deletion behavior is only exercised in GitHub Actions, not locally.
- Shell + `gh api` in workflow:
  - Pros: simple commands.
  - Cons: more quoting and pagination handling, less readable than a small JS block here.

## Selected Design

- Add a dedicated post-publish cleanup job using `actions/github-script`.
- Detect whether `github.repository_owner` is a user or organization before choosing the package API route.
- Keep the newest 10 versions and delete the rest after successful non-PR publishes.

## Validation Plan

- Parse the workflow YAML locally.
- Run doc placeholder and post-refactor scans for the scope docs.
- Run `git diff --check`.

## Risks and Assumptions

- Assumes the repository `GITHUB_TOKEN` can delete versions for the package it publishes.
- Assumes version `created_at` ordering matches the desired retention policy.
