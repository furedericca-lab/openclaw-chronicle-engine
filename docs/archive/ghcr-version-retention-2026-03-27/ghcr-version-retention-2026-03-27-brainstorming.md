description: Brainstorming and decision framing for enforcing GHCR backend image retention.
---

# ghcr-version-retention-2026-03-27 Brainstorming

## Problem

- The backend image package can grow without an enforced retention cap.
- The current repo does not have a built-in GHCR setting wired in code to retain only the newest N versions.

## Scope

- Add retention enforcement to the backend image workflow.
- Keep only the newest 10 package versions.
- Update active deploy/operator docs.

## Constraints

- Do not change image naming or publish triggers.
- Keep cleanup out of pull request runs.

## Options

- Option A: manual GHCR cleanup in the UI.
  - Rejected because it is not auditable or repeatable.
- Option B: post-publish cleanup inside the backend image workflow.
  - Chosen because it keeps the retention rule coupled to the image publisher.
- Option C: separate scheduled cleanup workflow.
  - Rejected because it introduces another automation surface and can drift from publish timing.

## Decision

- Add a post-publish cleanup job to `.github/workflows/docker-backend.yml`.
- Sort package versions by `created_at`, keep the newest 10, and delete the rest.

## Risks

- Local shell audit cannot confirm the current package count because the local `gh` token lacks `read:packages`.
- Workflow delete permissions depend on the package remaining admin-linked to this repository.

## Open Questions

- None for this bounded scope.
