---
description: Phase 4 closeout plan for verification, release safety, and post-deletion documentation convergence.
---

# Tasks: remote-authority-reset (Phase 4)

## Input
- `docs/remote-authority-reset/remote-only-local-authority-removal-plan.md`
- `README.md`
- `README_CN.md`
- `openclaw.plugin.json`
- `index.ts`
- `src/backend-client/*`
- `src/backend-tools.ts`
- `src/context/*`
- `test/*` (post-Phase-3 set)
- `docs/remote-authority-reset/*`

## Canonical architecture / Key constraints
- Closeout must prove one runtime authority model only.
- Docs/schema/test commands must match post-deletion reality.
- Archive history remains preserved and separate from active canonical docs.

## Format
- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 4 Goal
Finish release-ready convergence after local-authority deletion: verification evidence, final wording cleanup, and rollback-ready release notes.

Definition of Done:
- Regression suite and doc hygiene pass.
- User-facing docs/config no longer describe local-authority runtime operation.
- Release note + rollback instructions are explicit.

## Tasks
- [ ] T061 [QA] Run final regression set for remote-only runtime and context-engine orchestration.
  - DoD: all retained test suites pass; failures are triaged with blockers and owner.

- [ ] T062 [Docs] Clean residual wording drift in README/schema/help/canonical docs.
  - DoD: no active docs imply local-authority runtime support.

- [ ] T063 [P] [Docs] Run canonical doc hygiene and archive sanity checks.
  - DoD: placeholder scan, residual text scan, archive/canonical path inspection, and `git diff --check` all pass.

- [ ] T064 [Security] Re-confirm remote principal and backend-owned scope invariants post-deletion.
  - DoD: tool contracts still require runtime principal identity and do not accept client scope authority.

- [ ] T065 [Infra] Publish release-cut and rollback notes for deletion rollout.
  - DoD: release docs include stage-level rollback procedure and known upgrade breakages.

## Phase 4 Verification
```bash
npm test
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/remote-authority-reset
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/remote-authority-reset README.md
find docs/archive/2026-03-15-architecture-reset -maxdepth 2 -type f | sort
find docs/remote-authority-reset -maxdepth 2 -type f | sort
git diff --check
```

## Dependencies & Execution Order
- Phase 3 must be complete before Phase 4.
- `T061` should run before final wording pass in `T062` so docs reflect validated behavior.
- `T063` can run in parallel late in the phase.
- `T064` and `T065` are final sign-off steps.

Checkpoint:
- Remote-only deletion is fully verified, documented, and releasable with rollback discipline.
