---
description: Implementation research notes for the bundled admin plane, admin auth/logging wiring, and recall-flow tightening.
---

# backend-admin-plane-2026-03-27 Implementation Research Notes

## Baseline (Current State)

- The Rust backend already owns the entire supported authority surface:
  - persistence
  - retrieval and ranking
  - scope derivation and ACL
  - behavioral recall
  - transcript persistence
  - distill execution and artifact persistence
- The repository has no operator-facing admin UI or admin API layer today.
- `auth.admin` exists in config validation and example config but is not used by active routes.
- `logging.level` exists in config but no logger initialization consumes it.
- The backend Docker image currently builds only the Rust binary and has no frontend build stage.
- The recall handlers and recall pipeline have parallel generic/behavioral control flow in both `lib.rs` and `state.rs`.
- The LanceDB memory table is already guarded by strict compatibility rules, so broad schema churn there would raise migration risk for this scope.
- The LanceDB memory table already stores principal and access metadata that the admin plane can reuse directly:
  - `principal_user_id`
  - `principal_agent_id`
  - `access_count`
  - `last_accessed_at`
  - `behavioral_kind`
  - `strict_key`
- SQLite already stores distill jobs, distill artifacts, and transcript rows, so those surfaces are natural candidates for admin list/detail APIs.

## Gap Analysis

- Operators cannot browse principals, memories, recall traces, transcripts, or distill jobs through a supported UI.
- There is no dedicated admin-plane route namespace separated from `/v1/*`.
- Admin auth is declarative but not real.
- Structured logging/audit semantics are insufficient for an operator-facing management surface.
- The backend lacks list/query APIs for several human workflows, especially distill job history and transcript inspection.
- The current memory authority does not persist enough provenance to support the requested Memory Explorer filters and detail fields:
  - source
  - distill-derived flag
  - evidence/provenance drill-down
- Current runtime mutation APIs require `Actor` with `sessionId` and `sessionKey`, which is a mismatch for browser-driven admin memory editing if left unresolved.
- Current recall execution updates `access_count` and `last_accessed_at` for both ordinary and debug recall paths, so a naive admin Recall Lab implementation would pollute live retrieval metadata.
- Raw `user_id + \":\" + agent_id` or `{sessionKey}/{sessionId}` path encodings would be fragile because these values are not guaranteed to be delimiter-safe path tokens.
- Principal browsing cannot be sourced from memories alone because some principals may only have transcripts or distill jobs.

## Candidate Designs and Trade-offs

### Design A: Admin UI as a separate service

- Pros:
  - independent frontend release cadence
  - can use any auth stack
- Cons:
  - introduces a second deployable
  - encourages direct DB coupling or a second authority API surface
  - diverges from the repo’s single-backend-container deployment model

### Design B: Separate admin frontend app, but bundled and served by the Rust backend

- Pros:
  - keeps deployment single-process/single-container
  - keeps backend as the sole authority
  - makes route separation explicit: `/v1/*` vs `/admin/*`
  - easiest path to a usable operator console
- Cons:
  - backend Docker build becomes multi-stage across Rust + Node
  - backend must own static asset serving and SPA fallback behavior
  - backend must introduce explicit provenance/admin DTOs instead of reusing runtime request payloads blindly

### Provenance storage choice

- Option 1: add provenance columns to the LanceDB memory table.
  - Rejected for the first cut because the current backend treats schema compatibility seriously and this scope should avoid unnecessary LanceDB migration risk.
- Option 2: store provenance and audit events in SQLite companion tables keyed by `memory_id`.
  - Chosen because SQLite is already used for jobs/transcripts/idempotency and is a better fit for admin-oriented joins, history, and audit trails.
  - Recall-hit metadata stays in LanceDB because `access_count` and `last_accessed_at` already exist there.

### Identity encoding choice

- Option 1: raw path parameters such as `{userId}:{agentId}` or `{sessionKey}/{sessionId}`.
  - Rejected because delimiter collisions and path escaping would create brittle handlers and frontend route bugs.
- Option 2: opaque base64url-encoded canonical JSON ids.
  - Chosen because it keeps URLs stable without leaking delimiter assumptions into every handler.

### UI direction choice

- Option 1: invent a fresh dashboard language from scratch.
  - Rejected for the first cut because it increases design churn without improving operator clarity.
- Option 2: borrow the proven management-center interaction model from `/root/code/Cli-Proxy-API-Management-Center`.
  - Chosen as a reference for:
    - explicit login shell
    - left-nav + top-status layout
    - card-based dashboard
    - secondary edit shells
    - diff/confirmation modal flows
  - The scope reuses the interaction patterns, not the product branding.

### Design C: Server-rendered Rust-only admin HTML

- Pros:
  - no frontend toolchain
  - fewer build stages
- Cons:
  - much slower to evolve for filter-heavy operator workflows
  - poor fit for table/query-heavy administration pages
  - lower reuse for rich recall/debug/distill browsing

## Selected Design

- Choose Design B.
- Add `admin-web/` with:
  - React
  - TypeScript
  - Vite
  - TanStack Router
  - TanStack Query
  - TanStack Table
  - `shadcn/ui`-style component patterns
- Add backend admin modules under `backend/src/admin/` for:
  - auth
  - DTOs
  - routes
  - services
  - audit/log helpers as needed
- Keep `/v1/*` as the runtime plane.
- Add `/admin` + `/admin/assets/*` + `/admin/api/*` as the admin plane.
- Preserve the caller-scoped contract through “view as principal” admin APIs instead of a global unrestricted query surface.
- Introduce backend-owned admin services for mutations so the browser does not need to invent runtime actor/session fields.
- Persist provenance and audit information in backend-owned SQLite companion tables rather than deriving them only in the frontend.
- Use `tracing` + `tracing-subscriber` for logger wiring.
- Treat admin Recall Lab as a no-side-effect read path over the authority layer rather than a wrapper around existing side-effecting runtime recall handlers.
- Use principal-nested admin routes and opaque ids for principal and transcript selection.
- Keep bulk actions out of the first cut.
- Make Governance an interactive review/promote surface rather than a read-only artifact list.
- Make Settings an online config-editing surface with diff preview and validated writes.

## Validation Plan

- Backend tests:
  - existing `contract_semantics`
  - new admin-plane contract tests
- Frontend tests:
  - route smoke tests and selected component/data-hook tests
- Build tests:
  - admin-web production build
  - Rust backend build with bundled assets
  - Docker image build
- End-to-end checks:
  - `/v1/*` remains valid with runtime token
  - `/admin/api/*` rejects runtime token
  - `/admin/api/*` enforces admin rate limiting separately
  - `/admin` serves SPA shell
  - admin memory/recall/distill flows work against fixture data
  - admin recall simulation leaves `access_count` / `last_accessed_at` unchanged
  - principals with transcripts or distill jobs but no current memory rows still appear in principal selection

## Risks and Assumptions

- Assumes the MVP can use a backend admin bearer token without blocking future reverse-proxy SSO.
- Assumes the current SQLite sidecar store can absorb audit/event storage for the first cut if persistence is required.
- Assumes “view as principal” gives enough utility for the first operator release without introducing unrestricted cross-principal edits.
- Assumes provenance companion writes can be hooked into both runtime and admin mutation paths without regressing existing idempotent write behavior.
- Assumes the first cut can accept a public `/admin` shell plus authenticated `/admin/api/*` requests as long as the shell itself exposes no privileged data before login.
- Assumes online config editing can be made safe by atomic file writes plus explicit restart-required signaling even if not every setting is hot-reloadable.
