---
description: Task list for ghcr-version-retention-2026-03-27 phase 1.
---

# Tasks: ghcr-version-retention-2026-03-27 Phase 1

## Input
- Canonical sources:
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/README.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/ghcr-version-retention-2026-03-27/ghcr-version-retention-2026-03-27-scope-milestones.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/ghcr-version-retention-2026-03-27/ghcr-version-retention-2026-03-27-technical-documentation.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/ghcr-version-retention-2026-03-27/ghcr-version-retention-2026-03-27-contracts.md

## Canonical architecture / Key constraints
- Keep architecture aligned with ghcr-version-retention-2026-03-27 scope docs and contracts.
- Keep provider/runtime/channel boundaries unchanged unless explicitly in scope.
- Keep security and test gates in Definition of Done.

## Format
- [ID] [P?] [Component] Description
- [P] means parallelizable.
- Valid components: Backend, Frontend, Agentic, Docs, Config, QA, Security, Infra.
- Every task must have a clear DoD.

## Phase 1: GHCR Retention Enforcement
Goal: Add an auditable cleanup step that retains only the newest 10 backend image package versions in GHCR.

Definition of Done: The workflow deletes stale GHCR package versions after successful non-PR publishes, active docs describe the policy, and validation shows the workflow remains syntactically correct.

Tasks:
- [ ] T001 [Infra] Add a post-publish cleanup job to `.github/workflows/docker-backend.yml`.
  - DoD: The workflow keeps the newest 10 package versions for `chronicle-engine-backend`, deletes older versions, and skips cleanup on pull requests.
- [ ] T002 [Docs] Update active operator docs for the new retention behavior.
  - DoD: `deploy/README.md` and `docs/README.md` describe the retention rule and touched scope.
- [ ] T003 [Security] Keep publish and delete permissions bounded to the existing backend image workflow.
  - DoD: The workflow still scopes cleanup to the single backend container package and only runs after a successful non-PR publish.
- [ ] T004 [QA] Validate workflow syntax and scope docs.
  - DoD: YAML parse, doc placeholder scan, post-refactor scan, and `git diff --check` all pass.

Checkpoint: Phase 1 artifacts are merged, verified, and recorded in 1phases-checklist.md before next phase starts.

## Dependencies & Execution Order
- Phase 1 blocks all others.
- This phase must complete before any later phase starts.
- Tasks marked [P] within this phase may run concurrently only when they do not touch the same files.
