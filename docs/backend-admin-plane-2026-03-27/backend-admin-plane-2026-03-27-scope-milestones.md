---
description: Scope boundaries and milestones for the bundled admin plane and backend cleanup work.
---

# backend-admin-plane-2026-03-27 Scope and Milestones

## In Scope

- Admin-plane architecture and implementation docs.
- React + TypeScript single-page admin UI under `admin-web/`.
- Static asset serving from the Rust backend under `/admin` and `/admin/assets/*`.
- Admin-only JSON APIs under `/admin/api/*`.
- Real admin auth using `auth.admin`.
- Real logging initialization using `logging.level`.
- Separate admin-plane rate limiting.
- Admin audit/event logging for operator-visible actions.
- Provenance/source support for memory browsing and distill linkage.
- Principal-oriented browsing for:
  - memories
  - behavioral guidance
  - recall lab
  - distill jobs
  - transcripts
  - governance/session-lessons style artifacts
- Interactive governance review/promote actions.
- Online backend settings/config editing with validation and audit trail.
- Recall handler / recall pipeline duplication reduction where it helps the new admin plane.
- Docker/deploy updates needed to ship the admin UI inside the existing backend image.

## Out of Scope

- Replacing the Rust backend as authority.
- Introducing a separate admin microservice or second runtime container.
- Direct browser/database access to LanceDB or SQLite.
- Full SSO/OIDC implementation inside the backend for the MVP.
- A fully unrestricted cross-principal superuser query model by default.
- Reworking plugin-side runtime architecture beyond what is needed to document/administer the existing backend.

## Milestones

### Milestone 1 — Architecture freeze

- Contracts, milestones, and technical docs describe the admin-plane boundary clearly.
- Phase breakdown reflects backend, frontend, deploy, and verification work.

### Milestone 2 — Backend admin foundation

- Admin auth middleware is real.
- Logger setup is real.
- Admin rate limiting is real.
- `/admin` and `/admin/api/*` route families exist.
- Recall handler and pipeline duplication are reduced without semantic drift.
- A no-side-effect recall execution seam exists for admin simulation.
- The UI shell pattern and principal-selection model are frozen.

### Milestone 3 — Operator APIs

- Principal, memory, recall, distill, transcript, governance, and audit APIs exist in admin form.
- DTOs are explicit enough that the frontend does not need to infer storage schema.
- Memory provenance sync rules, transcript opaque ids, and principal-list union rules are explicit enough that the planned pages can render without hidden joins or ambiguous lookup rules.
- Governance review/promote and settings-config write flows are contractually defined.

### Milestone 4 — Admin SPA and bundling

- `admin-web/` builds successfully.
- The backend serves the SPA and assets from the same process/image.
- Core pages are usable against real backend APIs.

### Milestone 5 — Verification and release closeout

- Runtime plane and admin plane auth separation is verified.
- Docker/deploy flow remains single-container.
- Tests, builds, and docs all converge.

## Dependencies

- Depends on `runtime-architecture.md` remaining the canonical authority split.
- Depends on the backend remaining the only supported runtime authority.
- Depends on deploy staying single-container and HTTP-served.

## Exit Criteria

- Operators can browse and manage memories, recall traces, distill jobs, transcripts, and governance-relevant artifacts through the admin plane.
- Operators can review/promote governance artifacts and edit backend settings online with audit coverage.
- `auth.admin` and `logging.level` are live runtime inputs.
- Admin rate limiting and audit persistence are live runtime behaviors.
- `/v1/*` remains intact for ordinary OpenClaw runtime traffic.
- `/admin/*` is isolated from the runtime plane by route namespace and auth.
- The backend image includes the bundled admin UI and serves it successfully.
- Admin Recall Lab is observational only and does not distort runtime recall access metadata.
