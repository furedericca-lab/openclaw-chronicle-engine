description: Contracts for enforcing a 10-version GHCR retention policy for the backend image package.
---

# ghcr-version-retention-2026-03-27 Contracts

## Workflow Contracts

- The backend image workflow remains the only publisher for `ghcr.io/<owner>/chronicle-engine-backend`.
- Pull requests continue to build for validation only and must not delete package versions.
- Successful non-PR runs may delete stale package versions after the image push finishes.

## Package Retention Contracts

- Retention target: keep the newest `10` package versions for `chronicle-engine-backend`.
- Deletion target: any package version older than the newest `10`, regardless of tag shape.
- Sorting key: package `created_at` descending.
- Owner compatibility: the cleanup logic must work whether `github.repository_owner` is a user or an organization.

## Error Model

- If the image build or push fails, package cleanup must not run.
- If there are `10` or fewer package versions, cleanup must be a no-op.

## Validation and Compatibility Rules

- Do not change image naming, Docker build inputs, or publish triggers in this scope.
- Document the retention behavior in active deploy docs so operators know old versions are pruned automatically.
