description: Canonical technical architecture for GHCR backend image retention enforcement.
---

# ghcr-version-retention-2026-03-27 Technical Documentation

## Canonical Architecture

- The backend image publish workflow owns package retention for `ghcr.io/<owner>/chronicle-engine-backend`.
- Cleanup is a follow-on job after the image build/push job.

## Key Constraints and Non-Goals

- Cleanup must not run for pull requests.
- Cleanup must not affect any package other than `chronicle-engine-backend`.
- This scope does not change image tags or release semantics.

## Module Boundaries and Data Flow

- `build-image` pushes the container image.
- `cleanup-package-versions` runs only after `build-image` succeeds on non-PR events.
- The cleanup job lists package versions, sorts by `created_at`, keeps the newest 10, and deletes the remainder.

## Interfaces and Contracts

- The cleanup job uses GitHub REST package-version APIs.
- Owner path selection is dynamic so the workflow works for both user-owned and org-owned repositories.

## Security and Reliability

- Cleanup is bounded to one package name.
- Cleanup is no-op when there are 10 or fewer versions.
- If build/push fails, the cleanup job does not run.

## Test Strategy

- Local validation focuses on workflow YAML parsing and documentation scans.
- Runtime deletion behavior is exercised by the next successful non-PR publish in GitHub Actions.
