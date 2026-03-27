---
description: Canonical technical architecture for the bundled backend admin plane.
---

# backend-admin-plane-2026-03-27 Technical Documentation

## Canonical Architecture

Chronicle Engine continues to ship one supported authority runtime:

1. `backend/` remains the only authority for:
   - memory persistence
   - retrieval/ranking
   - scope derivation and ACL
   - behavioral guidance recall
   - transcript persistence
   - distill execution and artifact persistence
2. The admin plane is an operator-facing surface attached to that same authority backend.
   - `/admin` serves the SPA shell.
   - `/admin/assets/*` serves static assets.
   - `/admin/api/*` exposes admin-only orchestration and query routes.
3. `/v1/*` stays the ordinary runtime data plane for OpenClaw/plugin callers.

The admin plane is explicitly not a second authority. It is a new operations surface attached to the same backend.

## Key Constraints and Non-Goals

- The admin UI must not query LanceDB or SQLite directly.
- The admin plane must keep auth, auditing, and rate-limiting logically separate from the runtime plane.
- The first cut prioritizes operator visibility and bounded mutation workflows rather than every possible backend feature.
- The principal model remains explicit: operators pick a principal context and operate within it rather than browsing the whole system as an implicit unrestricted global user.
- The MVP chooses bearer-header auth for `/admin/api/*` instead of cookie auth so the first cut avoids CSRF/session complexity.
- Admin Recall Lab requests must be observational only; operator simulations must not mutate recall access metadata used by ranking and observability.

## Module Boundaries and Data Flow

### Backend modules

- `backend/src/lib.rs`
  - root router composition
  - runtime-plane middleware
  - admin-plane middleware composition
  - static asset serving
- `backend/src/admin/auth.rs`
  - admin auth extraction and verification
  - admin rate-limiter identity extraction
- `backend/src/admin/routes.rs`
  - `/admin/api/*` route registration and handler entrypoints
- `backend/src/admin/dto.rs`
  - admin-specific response/request DTOs
- `backend/src/admin/service.rs`
  - aggregation and translation over the existing authority stores/repos
  - opaque id parsing for `principalId` and `transcriptId`
  - admin mutation helpers that do not depend on runtime-only actor/session fields
- `backend/src/state.rs`
  - remains the authority implementation for memory, retrieval, distill, transcript, and persistence
  - gains narrowly-scoped list/query helpers where admin APIs need new access patterns
  - gains companion-store hooks for provenance and admin audit tracking
  - gains a side-effect-free recall execution seam for admin simulation so operator debugging does not increment runtime access metadata
- `backend/src/config.rs`
  - `auth.admin` and `logging.level` become active runtime inputs
- `backend/src/main.rs`
  - initializes `tracing` / `tracing-subscriber` from `logging.level`

### Frontend modules

- `admin-web/src/routes/*`
  - route layout and page entrypoints
- `admin-web/src/features/memories/*`
- `admin-web/src/features/recall/*`
- `admin-web/src/features/distill/*`
- `admin-web/src/features/transcripts/*`
- `admin-web/src/features/governance/*`
- `admin-web/src/lib/api/*`
  - typed admin API client
- `admin-web/src/components/*`
  - tables, drawers, filters, detail panes
  - confirmation and diff modals

### Deployment flow

1. Build `admin-web/` into static assets.
2. Copy those assets into the backend runtime image.
3. Start the single `chronicle-engine-rs` process.
4. Backend serves:
   - runtime JSON APIs at `/v1/*`
   - admin JSON APIs at `/admin/api/*`
   - admin SPA at `/admin`

### Provenance and audit persistence

- Memory provenance and admin audit metadata live in SQLite companion tables keyed by `memory_id` or audit event id.
- Recall-hit metadata does not move into SQLite because LanceDB memory rows already carry `access_count` and `last_accessed_at`.
- This keeps the LanceDB schema focused on retrieval while still avoiding avoidable migration churn in this scope.
- Distill artifacts and persisted memory links remain backend-owned and are exposed through admin DTOs rather than raw table access.
- Admin audit events are persisted in SQLite and also emitted through structured logs.

### Principal and transcript identity model

- The current backend persists principal ownership in multiple stores:
  - LanceDB memory rows via `principal_user_id` / `principal_agent_id`
  - `session_transcript_messages`
  - `distill_jobs`
- The admin plane therefore treats principal selection as a first-class route concern.
- `principalId` and `transcriptId` are opaque route identifiers encoded from canonical JSON payloads rather than delimiter-joined raw strings.
- Transcript browsing is principal-nested and transcript-detail routes resolve the opaque transcript token back to `{sessionKey, sessionId}` before querying SQLite.

### Current backend data realities that shape the admin API

- Memory rows already expose:
  - `category`
  - `behavioral_kind`
  - `strict_key`
  - `created_at`
  - `updated_at`
  - `access_count`
  - `last_accessed_at`
  - owner principal columns
- Distill jobs and transcript rows already live in SQLite and are better suited than LanceDB for list/detail/operator history surfaces.
- Current runtime list/stats APIs are intentionally too narrow for the admin plane:
  - they do not expose provenance
  - they do not expose transcript heads
  - they do not expose distill history browsing
- Current recall routes always record access metadata, including debug recall. Admin simulation must therefore use an explicit no-side-effect backend seam rather than simply wrapping existing `/v1/debug/recall/*` handlers.

### Admin UI composition and style direction

- The UI direction should explicitly borrow the clearer operator ergonomics of `/root/code/Cli-Proxy-API-Management-Center`:
  - split login shell
  - fixed left navigation
  - sticky top action/status bar
  - dashboard card grid
  - secondary shells for deep edit/detail screens
  - explicit confirmation and diff modals before destructive or config-changing writes
- Chronicle Engine should not clone that project’s brand or page taxonomy blindly.
- The reusable pattern is operational clarity:
  - fast recognition of current environment and status
  - predictable navigation
  - dense but readable tables
  - isolated edit flows with safe review before apply

### Page-by-page interaction direction

- Login
  - Borrow the split-shell login treatment from the management-center:
    - one side for Chronicle Engine brand/context
    - one side for token-entry and connection state
  - Keep the login screen minimal and operational, not decorative.
  - Surface the selected backend endpoint, auth mode, and recent principal hinting after successful connection.
- Dashboard
  - Borrow the management-center dashboard card-grid pattern.
  - Use a quick-stat first screen with:
    - total memories
    - behavioral row count
    - recent 24h writes
    - failed distill jobs
    - recent active principals
  - Keep cards actionable so operators can jump directly into Memories, Distill Jobs, or Audit Log.
- Memories
  - Borrow the dense table + secondary detail shell pattern.
  - Primary screen should stay a filterable table with quick row actions, while edit/detail flows open in a secondary shell or drawer rather than navigating to a blank standalone page.
  - Use confirmation modals for destructive deletes.
- Behavioral
  - Reuse the same table/detail shell grammar as Memories, but visually emphasize strict keys and behavioral kinds the way the management-center emphasizes provider-specific metadata.
  - This page should feel like a specialized operational lane, not a duplicate generic memory list.
- Recall Lab
  - Borrow the management-center “configuration on the left, results on the right” troubleshooting rhythm.
  - The page should present query controls, resolved principal, and filter summary above a two-panel result area:
    - recall results
    - debug trace
  - Trace stages should read like diagnostic cards, not raw JSON dumps.
- Distill Jobs
  - Borrow the management-center status-board treatment:
    - list view with strong status badges
    - inline summary metrics
    - detail shell for artifacts/result payloads
  - Failed jobs should surface retryability and error summaries prominently in the list itself.
- Transcripts
  - Borrow the secondary-screen shell pattern for drill-down.
  - Use a compact transcript head list first, then a focused detail screen that reads like a chronological inspection surface rather than a generic table.
- Governance
  - Borrow the management-center review/edit cadence:
    - artifact list
    - detail shell
    - explicit review action buttons
    - confirmation modal for promote
  - Review state, reviewer note, and promotion outcome should be visible without opening raw payloads.
- Audit Log
  - Borrow the management-center logs-style operator workflow:
    - filter bar at top
    - dense chronological table/list
    - expandable detail panel for structured payloads
  - Keep it optimized for triage and traceability, not visual polish.
- Settings
  - Borrow the management-center config workflow most directly here:
    - structured settings sections for common values
    - raw editor/review area for advanced values
    - diff modal before apply
    - clear save/apply feedback
  - This page should feel like an operator control plane, not a plain form dump.

### Principal selection model

- The selected principal is a first-class app state, not an incidental filter chip.
- After login, Dashboard may render global backend/admin health plus a recent-principals panel.
- Principal-specific pages require an active principal in route/app state.
- The principal switcher lives in the top bar and supports:
  - search
  - recent principals from `sessionStorage`
  - server-ranked suggestions by recent activity
- The first-cut principal ranking heuristic is:
  - max activity timestamp across memories, transcripts, and distill jobs
  - then memory count
  - then transcript count
  - then distill job count
  - then lexical `userId` / `agentId`

## Interfaces and Contracts

### Runtime plane

- `/v1/*` remains optimized for OpenClaw/plugin callers and preserves existing request semantics.

### Admin plane

- `/admin/api/*` uses admin auth and admin DTOs.
- Admin APIs may aggregate multiple backend stores/repo calls into one operator-friendly response.
- Admin APIs may expose list and detail views that do not exist in the runtime plane, but they must still use the same underlying backend authority logic.
- The admin plane uses dedicated mutation adapters rather than requiring the browser to send runtime-only actor/session fields.
- Admin mutation routes keep idempotency semantics by requiring `Idempotency-Key` and storing reservations under admin-plane operation names.

### View-as-principal contract

- The admin UI selects a principal first.
- Memory views, recall simulation, transcript browsing, and most mutations are scoped to that selected principal.
- This allows the admin plane to remain aligned with the current repository contract instead of silently creating a global bypass surface.

### Principal-nested route layout

- Principal-scoped admin families use nested routes rather than mixed global routes:
  - `/admin/api/principals/{principalId}/memories/*`
  - `/admin/api/principals/{principalId}/recall/simulate`
  - `/admin/api/principals/{principalId}/distill/jobs/*`
  - `/admin/api/principals/{principalId}/session-transcripts/*`
  - `/admin/api/principals/{principalId}/governance/artifacts`
- This keeps selected-principal context in the URL and avoids duplicating it inconsistently across body, query, and local UI state.

### Governance and behavioral views

- Governance pages are backed by `distill_artifacts`, not by the memory table:
  - lessons
  - governance candidates
  - follow-up focus / next-turn guidance subtypes
- Governance is an interactive review surface in this version:
  - approve / dismiss review state
  - reviewer note
  - optional promote-to-memory action when legal
- Behavioral pages are backed by the memory authority:
  - `category=behavioral`
  - `behavioral_kind`
  - `strict_key`
- These two views must stay separate so operators do not confuse persisted memory rows with upstream distill artifacts.

### Settings page behavior

- Settings is an active online configuration surface in this version, not a placeholder.
- The page should combine:
  - parsed form sections for common backend settings
  - raw TOML review/edit for advanced cases
  - explicit diff preview before save
- Config writes validate against backend config parsing before file replacement.
- The backend returns whether the saved change is hot-applied or restart-required.
- Unsafe config writes must fail closed without partially mutating the file on disk.

## Security and Reliability

- `auth.runtime` continues to guard `/v1/*`.
- `auth.admin` guards `/admin/api/*`.
- `/admin` may remain a public shell for token-entry bootstrap as long as it exposes no privileged data before the first authenticated API call.
- `/admin/api/*` uses its own rate limiter, separate from runtime-plane traffic.
- Admin requests must be auditable.
- Runtime and admin credentials must not be interchangeable.
- Admin-plane failures must not degrade runtime-plane request handling.
- Static asset serving must not shadow `/v1/*` or `/admin/api/*`.
- The admin SPA uses same-origin fetches with bearer headers; no CORS support is required for the MVP.
- The login flow is a lightweight shell over `/admin`:
  - the SPA can load unauthenticated
  - the browser stores the admin token in `sessionStorage`
  - all `/admin/api/*` fetches attach the bearer header
  - refreshes survive within the tab/session but do not persist across browser restarts
- Unknown `/admin/api/*` routes must stay JSON `404`; SPA fallback applies only to non-asset client routes under `/admin`.
- Destructive governance/settings actions must require confirmation modals and produce auditable outcomes.

## Test Strategy

### Backend

- Extend backend contract coverage with admin-plane tests:
  - auth separation
  - rate limiting
  - principal listing
  - memory explorer list/detail/mutation
  - recall simulate
  - distill job listing/detail
  - transcript listing/detail
  - provenance and audit behavior

### Frontend

- Route smoke tests
- API client tests
- Page-level tests for:
  - Memory Explorer
  - Recall Lab
  - Distill Job Center
  - Governance review/promote flows
  - Settings diff-and-save flow

### Build/deploy

- Vite production build
- Rust backend build with embedded/static asset path
- Docker image build with combined Rust + frontend stages
- Static asset fallback verification for `/admin` without intercepting `/admin/api/*`
- Standalone `admin-web/package.json` and lockfile aligned with the repository's existing npm-based tooling rather than introducing a workspace manager in this scope
- Recall Lab verification that confirms `access_count` / `last_accessed_at` are unchanged after simulation
