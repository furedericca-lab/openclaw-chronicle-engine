---
description: Brainstorming and decision framing for adding a bundled admin plane to the Rust backend authority.
---

# backend-admin-plane-2026-03-27 Brainstorming

## Problem

- The repository has a strong runtime data plane but no supported operator-facing management surface.
- Operators currently need direct route calls, ad hoc storage inspection, or test-only helpers to inspect memories, replay recall behavior, and review distill artifacts.
- `auth.admin` and `logging.level` already exist in config, but neither is connected to live runtime behavior.
- The recall handlers and recall pipeline contain parallel generic/behavioral control flow that will become harder to evolve once an interactive admin plane starts exercising those paths continuously.

## Scope

- Add a bundled admin plane that remains fully dependent on the existing Rust backend authority.
- Keep `/v1/*` as the runtime data plane for OpenClaw/plugin callers.
- Add `/admin` and `/admin/api/*` for operator workflows.
- Bundle a React + TypeScript SPA into the backend deployment artifact so the single-container model remains intact.
- Wire `auth.admin` into actual admin-plane authentication.
- Wire `logging.level` into actual backend logging setup.
- Tighten duplicated recall handler and retrieval-pipeline structure without changing recall semantics or authority boundaries.

## Constraints

- The Rust backend remains the only authority for persistence, retrieval, ranking, ACL/scope, behavioral recall, distill execution, transcript persistence, and artifact persistence.
- The admin plane must not become a second authority or a direct database client.
- Admin endpoints must stay off the ordinary runtime path and use separate auth.
- The first version should favor operator usefulness over completeness.
- The principal model should remain explicit: operators select a principal context rather than browsing the whole system through an implicit bypass.

## Options

- Option A: separate admin service that talks directly to LanceDB/SQLite.
  - Rejected because it would split authority, duplicate schema knowledge, and diverge from the single-container deployment shape.
- Option B: separate admin frontend service plus the existing backend.
  - Rejected for the first cut because it complicates deployment and still creates a second routable service surface.
- Option C: bundled SPA served by the existing Rust backend with new `/admin/api/*` routes.
  - Chosen because it preserves the backend as sole authority, matches the current deployment model, and keeps admin/data-plane layering explicit.

## Decision

- Build a bundled admin plane inside the existing backend process and image.
- Add `admin-web/` with React, TypeScript, and Vite-based SPA tooling.
- Serve the SPA from `/admin` and static assets from `/admin/assets/*`.
- Add `/admin/api/*` routes guarded by admin auth middleware and separate audit logging.
- Preserve `/v1/*` unchanged as the ordinary runtime data plane.
- Start with a “view as principal” interaction model so the admin UI remains aligned with caller-scoped contracts.
- Keep the MVP auth model simple and explicit: a login screen collects the backend admin bearer token and the browser sends it on `/admin/api/*` as `Authorization: Bearer <token>`, leaving room for later reverse-proxy SSO/OIDC in front.
- Persist admin audit events in SQLite and also mirror them to structured logs.
- Add explicit memory provenance records in SQLite companion tables so source and distill derivation can be rendered without risky LanceDB schema churn, while leaving recall-hit metadata on the existing LanceDB access fields.
- Use explicit admin-side rate limiting for `/admin/api/*`, separated from the runtime plane.
- Use opaque base64url JSON route ids for principals and transcript heads rather than delimiter-joined raw identifiers.
- Treat Recall Lab as a no-side-effect backend read path so operator simulations do not affect live access counters or ranking metadata.
- Do not include bulk actions in the first cut.
- Borrow page structure and operator workflows from `/root/code/Cli-Proxy-API-Management-Center`: login shell, left nav, top status bar, stat cards, secondary edit shells, and diff/confirm modals.
- Make Governance and Settings active operator surfaces rather than read-only placeholders.

## Risks

- The scope is broad: backend routing, storage access patterns, frontend build tooling, deploy image changes, and auth/logging all move together.
- Admin list/detail APIs could accidentally widen visibility or bypass principal semantics if the contracts are underspecified.
- A naive UI can turn into a schema-guessing frontend. Backend admin DTOs need to be explicit so the frontend does not reverse-engineer internal storage structures.
- Static asset serving inside the backend can create deployment complexity if the Docker build is not updated carefully.
- If provenance sync is only added to admin mutations and not ordinary runtime writes, Memory Explorer data will drift immediately.

## Open Questions

- No product-level open question currently blocks Phase 2-4 execution.
