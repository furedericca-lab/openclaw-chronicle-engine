---
description: Task list for backend-admin-plane-2026-03-27 phase 4.
---

# Tasks: backend-admin-plane-2026-03-27 Phase 4

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

## Phase 4: Bundled Admin Web UI
Goal: Build and integrate the React + TypeScript admin SPA against the new admin APIs.

Definition of Done: The admin UI builds, is served by the backend from `/admin`, and exposes the core operator pages needed for day-one administration.

Tasks:
- [ ] T061 [Frontend] Scaffold `admin-web/` with Vite, React, TypeScript, TanStack Router, TanStack Query, and table/layout foundations.
  - DoD: the app builds and has route/layout structure for the admin plane, borrowing the management-center interaction model of login shell, persistent sidebar, top status bar, and secondary edit shells.
- [ ] T062 [Frontend] Implement the core pages.
  - DoD: Dashboard, Memories, Behavioral, Recall Lab, Distill Jobs, Transcripts, Governance, Audit Log, and Settings have usable initial views, with Governance review/promote actions and Settings diff-and-save config editing.
- [ ] T063 [Frontend] Add admin API client and page-level state wiring.
  - DoD: pages consume typed `/admin/api/*` responses rather than in-page schema guessing, and the login shell stores the admin token in `sessionStorage` for same-origin bearer fetches.
- [ ] T064 [QA] Add frontend smoke/route tests and static-asset integration checks.
  - DoD: route smoke tests pass and backend can serve the built SPA shell and assets.

Checkpoint: Phase 4 artifacts are merged, verified, and recorded in 5phases-checklist.md before next phase starts.

## Dependencies & Execution Order
- Phase 1 blocks all others.
- Phase 4 depends on completion of phases 1-3.
- Tasks marked [P] within this phase may run concurrently only when they do not touch the same files.
