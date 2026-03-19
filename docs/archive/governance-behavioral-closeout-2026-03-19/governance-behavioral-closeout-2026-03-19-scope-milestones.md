---
description: Scope boundaries and milestones for governance-behavioral-closeout-2026-03-19.
---

# governance-behavioral-closeout-2026-03-19 Scope and Milestones

## In Scope

- Remove legacy governance tool aliases and the dead `self-improvement` module surface.
- Delete wrapper/shim files for old reflection/self-improvement naming and update imports/tests/docs to canonical modules.
- Rename adapter/client/backend internal helpers toward behavioral-guidance wording where it does not churn the stable backend route/storage contract.
- Update `README.md`, `README_CN.md`, `docs/runtime-architecture.md`, `docs/README.md`, and `docs/archive-index.md`.
- Archive `docs/autorecall-governance-unification-2026-03-18/` under `docs/archive/`.
- Record concrete verification commands and outcomes in this scope doc set.

## Out of Scope

- Renaming backend HTTP routes or persisted category/storage fields away from `reflection`.
- Rewriting every historical design snapshot outside this scope.
- Backfilling a migration helper for legacy `.learnings/` workspace directories.
- Any change to distill ownership, remote-authority boundaries, or session transcript cadence semantics.

## Milestones

- M1: Contract/baseline docs rewritten from scaffold placeholders into real scope-specific docs. Status: completed.
- M2: Governance surface canonicalized.
  - Remove `self_improvement_*` aliases.
  - Delete `src/self-improvement-tools.ts`.
  - Delete `.learnings` read-through compatibility.
  Status: completed.
- M3: Neutral internal behavioral-guidance naming applied where safe.
  - Adapter/client/tool internals renamed.
  - Backend helper/handler names renamed while keeping route/storage contract stable.
  Status: completed.
- M4: Archive and verification closeout completed.
  - Previous unification scope moved under `docs/archive/`.
  - Targeted JS tests, full JS suite, backend test binary, and doc scans recorded.
  - Residual live matches reduced to intentional alias-rejection guards/tests only.
  Status: completed.

## Dependencies

- Local Node dev dependencies from `package-lock.json` (`npm ci`) to run the JS suite in this worktree.
- Cargo toolchain for `backend/`.
- Repo-task-driven doc validation scripts under `/root/.openclaw/workspace/skills/repo-task-driven/scripts/`.

## Exit Criteria

- No active tool/module/doc surface advertises `self_improvement_*`.
- No active wrapper module remains for reflection/self-improvement naming.
- Plugin/runtime docs describe governance-only and behavioral-guidance-only semantics.
- The old unification scope is archived and indexed.
- JS verification passes, backend verification passes, and doc scans are clean.
- Remaining boundary is explicitly documented:
  - backend route/storage contract still uses `reflection`;
  - active canonical wording does not.
