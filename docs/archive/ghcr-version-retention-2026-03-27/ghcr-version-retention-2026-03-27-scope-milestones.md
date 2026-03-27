description: Scope boundaries and milestones for enforcing GHCR package retention on the backend image workflow.
---

# ghcr-version-retention-2026-03-27 Scope and Milestones

## In Scope

- Add a post-publish GHCR cleanup job to `.github/workflows/docker-backend.yml`.
- Retain only the newest 10 versions of `chronicle-engine-backend`.
- Update active deploy/operator docs for the retention rule.

## Out of Scope

- Renaming the container image.
- Changing publish triggers, tags, or Docker build inputs.
- Auditing or rewriting historical GHCR package metadata outside the workflow.

## Milestones

- Milestone 1: workflow cleanup logic added and bounded to successful non-PR publishes.
- Milestone 2: active docs updated to describe the retention policy.
- Milestone 3: local YAML/doc verification passes.

## Dependencies

- Depends on the existing `docker-backend.yml` publish flow remaining the single image publisher.

## Exit Criteria

- Workflow syntax is valid.
- The workflow deletes versions older than the newest 10 after successful non-PR publishes.
- Active docs mention the 10-version retention behavior.
