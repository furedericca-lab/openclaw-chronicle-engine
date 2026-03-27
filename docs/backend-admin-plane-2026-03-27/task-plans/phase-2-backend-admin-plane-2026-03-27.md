---
description: Task list for backend-admin-plane-2026-03-27 phase 2.
---

# Tasks: backend-admin-plane-2026-03-27 Phase 2

## Input
- Canonical sources:
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/README.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/backend-admin-plane-2026-03-27/backend-admin-plane-2026-03-27-scope-milestones.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/backend-admin-plane-2026-03-27/backend-admin-plane-2026-03-27-technical-documentation.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/backend-admin-plane-2026-03-27/backend-admin-plane-2026-03-27-contracts.md

## Canonical architecture / Key constraints
- Keep architecture aligned with backend-admin-plane-2026-03-27 scope docs and contracts.
- Keep provider/runtime/channel boundaries unchanged unless explicitly in scope.
- Keep security and test gates in Definition of Done.

## Format
- [ID] [P?] [Component] Description
- [P] means parallelizable.
- Valid components: Backend, Frontend, Agentic, Docs, Config, QA, Security, Infra.
- Every task must have a clear DoD.

## Phase 2: Backend Admin Foundation
Goal: Add backend routing, auth, logging, and shared recall-flow refactors needed for the admin plane.

Definition of Done: The backend exposes an admin namespace with real admin auth, real logging initialization, and a cleaned-up recall entry structure without changing runtime-plane semantics.

Tasks:
- [ ] T021 [Backend] Add backend admin modules and router composition.
  - DoD: `backend/src/admin/*` exists with route/auth/DTO/service scaffolding and `backend/src/lib.rs` cleanly mounts `/admin` and `/admin/api/*`.
- [ ] T022 [Backend] Wire `auth.admin` and `logging.level` into live runtime behavior.
  - DoD: admin middleware authenticates with `auth.admin`, backend logging initializes according to `logging.level`, and the admin-shell login bootstrap does not require cookie auth.
- [ ] T023 [Backend] Add dedicated admin-plane rate limiting and principal parsing seams.
  - DoD: `/admin/api/*` has separate rate limiting plus centralized opaque id parse/validate paths for `principalId` and `transcriptId`.
- [ ] T024 [Backend] Consolidate duplicate recall handler / pipeline control flow.
  - DoD: generic and behavioral recall paths share more implementation while preserving current request/response semantics and trace behavior, and admin simulation has a no-side-effect execution path.
- [ ] T025 [QA] Add backend tests for admin-vs-runtime auth separation and static/admin route behavior.
  - DoD: tests cover rejection of runtime token on admin routes, rejection of admin token on runtime routes where intended, and SPA/static route handling.

Checkpoint: Phase 2 artifacts are merged, verified, and recorded in 5phases-checklist.md before next phase starts.

## Dependencies & Execution Order
- Phase 1 blocks all others.
- Phase 2 depends on completion of phase 1.
- Tasks marked [P] within this phase may run concurrently only when they do not touch the same files.
