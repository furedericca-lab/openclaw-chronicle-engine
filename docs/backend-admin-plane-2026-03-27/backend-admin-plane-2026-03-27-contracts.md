---
description: API, auth, and deployment contracts for the bundled backend admin plane.
---

# backend-admin-plane-2026-03-27 Contracts

## API Contracts

### Data-plane preservation

- `/v1/*` remains the only runtime data plane for OpenClaw/plugin traffic.
- Existing runtime route families remain caller-scoped and principal-aware:
  - `/v1/memories/*`
  - `/v1/recall/*`
  - `/v1/debug/recall/*`
  - `/v1/session-transcripts/append`
  - `/v1/distill/jobs`
- This scope may refactor shared implementation under those routes, but must not widen accepted inputs or change authority semantics.

### Admin-plane routes

- `/admin`
  - serves the SPA entry document
  - returns `index.html` for non-asset client-side routes beneath `/admin/*`
- `/admin/assets/*`
  - serves built SPA assets
  - must not shadow `/admin/api/*`
- `/admin/api/*`
  - serves admin-only JSON APIs guarded by admin auth

### MVP admin API families

- `GET /admin/api/principals`
- `GET /admin/api/principals/{principalId}/memories`
- `GET /admin/api/principals/{principalId}/memories/{memoryId}`
- `POST /admin/api/principals/{principalId}/memories`
- `PATCH /admin/api/principals/{principalId}/memories/{memoryId}`
- `DELETE /admin/api/principals/{principalId}/memories/{memoryId}`
- `POST /admin/api/principals/{principalId}/recall/simulate`
- `GET /admin/api/principals/{principalId}/distill/jobs`
- `GET /admin/api/principals/{principalId}/distill/jobs/{jobId}`
- `GET /admin/api/principals/{principalId}/session-transcripts`
- `GET /admin/api/principals/{principalId}/session-transcripts/{transcriptId}`
- `GET /admin/api/principals/{principalId}/governance/artifacts`
- `POST /admin/api/principals/{principalId}/governance/artifacts/{artifactId}/review`
- `POST /admin/api/principals/{principalId}/governance/artifacts/{artifactId}/promote`
- `GET /admin/api/settings/runtime-config`
- `PUT /admin/api/settings/runtime-config`
- `GET /admin/api/audit-log`

## Shared Types / Schemas

### Principal-first admin model

- The admin plane operates in an explicit “view as principal” mode.
- `principalId` is a stable admin-plane route identifier derived as URL-safe base64url of canonical JSON:
  - `{"userId":"...","agentId":"..."}`
- The route identifier must not rely on delimiter-joined raw strings because `user_id` and `agent_id` may legally contain characters such as `:`.
- Admin DTOs must expose both the route identifier and the original `userId` / `agentId` fields.
- Admin services must parse and validate `principalId` centrally rather than letting handlers split raw strings ad hoc.
- `GET /admin/api/principals` must derive its list from the union of backend-owned principal-bearing stores:
  - LanceDB memory rows
  - `session_transcript_messages`
  - `distill_jobs`
- The list should sort by most recent activity so principals with transcripts/jobs but no current memory rows still appear.
- The principal ranking heuristic for the first cut is:
  - primary sort: max of memory activity, transcript activity, and distill activity descending
  - tie-break 1: memory count descending
  - tie-break 2: transcript count descending
  - tie-break 3: distill job count descending
  - final tie-break: `userId`, then `agentId`
- The SPA keeps a short recent-principals list in `sessionStorage` so the operator can switch quickly after login.

### Admin auth transport

- The SPA shell may be publicly reachable under `/admin`, but `/admin/api/*` must require the admin bearer token.
- The browser sends the admin token in the standard `Authorization: Bearer <token>` header.
- The SPA presents an explicit login gate and stores the token in `sessionStorage`, not cookies and not `localStorage`.
- The admin token is not stored in cookies in the MVP, which keeps the admin plane out of CSRF-sensitive cookie semantics.
- The runtime bearer token must never be accepted on `/admin/api/*`.
- The admin bearer token must never be accepted on ordinary `/v1/*` data-plane routes.

### Admin rate limiting

- `/admin/api/*` must use a dedicated admin-plane rate limiter.
- The initial limiter may be in-memory and per remote IP plus token fingerprint.
- Rate-limit failures must return explicit JSON `429` responses.
- Runtime-plane rate behavior must remain independent.

### Memory provenance schema

- To support source/provenance browsing, the authority layer must persist explicit provenance records rather than inferring source from ad hoc joins only.
- The first cut stores provenance in SQLite companion tables keyed by `memory_id`, not by expanding the LanceDB schema.
- The provenance store must expose fields equivalent to:
  - `source_kind`
  - `source_ref`
  - `source_label`
  - `source_detail_json`
- For distill-derived rows, provenance must also be able to carry the originating `job_id` and `artifact_id`.
- Existing LanceDB `access_count` and `last_accessed_at` remain the source of truth for recall-hit metadata; the admin plane must not introduce a second competing `last_recalled_at` field.
- Provenance is backend-owned implementation detail and may be absent from `/v1/*` until intentionally exposed there.
- Provenance rows must be kept in sync for both runtime-plane and admin-plane mutations:
  - runtime `tool-store`
  - runtime `auto-capture`
  - backend-owned distill persistence
  - admin create/update/delete flows

### Memory explorer DTOs

- Memory list rows must include at minimum:
  - `id`
  - `principal`
  - `textPreview`
  - `category`
  - `behavioralKind` when applicable
  - `scope`
  - `createdAt`
  - `updatedAt`
  - `accessCount`
  - `lastAccessedAt`
  - `source`
  - `isBehavioral`
  - `isDistillDerived`
- Memory detail DTOs must include:
  - full text
  - category/subtype
  - provenance/source details
  - timestamps
  - persisted strict key / behavioral metadata when applicable
  - related evidence/provenance fields when present
- Admin mutation APIs must not require the browser to provide runtime-only actor fields such as `sessionId` or `sessionKey`.
- Admin services may adapt selected principal context into backend-owned write/update/delete helpers internally.
- Admin mutation routes (`POST` / `PATCH` / `DELETE`) require `Idempotency-Key` just like the runtime write surface, but persist under separate admin-plane operation names.

### Recall simulation DTOs

- One admin request shape should cover:
  - selected principal
  - generic vs behavioral mode
  - query
  - topK / limit
  - filters such as categories, age window, excludeBehavioral, includeKinds, minScore
- One admin response shape should return:
  - resolved principal
  - recall results
  - debug trace payload
  - selected mode and applied filter summary
- Admin recall simulation must be side-effect-free:
  - it must not update `access_count`
  - it must not update `last_accessed_at`
  - it must not emit provenance mutations or distill side effects beyond optional read-only structured logs

### Audit log DTOs

- Every admin mutation route must emit audit entries with at minimum:
  - timestamp
  - admin subject
  - action
  - target principal
  - target resource kind/id
  - outcome
- Persisted admin audit rows should also carry a `details_json` payload so later UI detail views do not depend on raw log scraping.
- Read-only admin routes should emit lightweight structured logs even if not all are persisted in the first cut.
- The MVP keeps a persisted admin audit table in SQLite so the admin UI can browse recent actions without log scraping.

### Governance and settings action DTOs

- Governance is not read-only in this version.
- Governance review routes must support at minimum:
  - `reviewStatus` such as `approved` / `dismissed`
  - optional reviewer note
  - reviewer identity and timestamp
- Governance promote routes may persist a reviewed artifact as an ordinary memory row only when that write does not violate the backend-managed behavioral write restriction.
- Governance review state must be persisted in SQLite so the page does not depend on transient frontend state.
- Settings routes must support online config editing of the backend TOML source of truth.
- `PUT /admin/api/settings/runtime-config` must:
  - validate the proposed config before writing
  - return a structured diff/summary
  - write atomically
  - emit audit entries
  - indicate whether a restart is required for the change to take effect

### Distill and transcript DTOs

- Distill job detail responses for the admin plane must include associated artifacts inline or via a clearly adjacent field so the Distill Job Center does not require N+1 route calls per job row.
- Distill list/detail routes are principal-scoped routes even though the underlying SQLite store already records owner principal columns.
- Transcript head list responses must expose:
  - `principal`
  - `sessionKey`
  - `sessionId`
  - message count
  - first/last timestamp
- `transcriptId` is an opaque route identifier derived as URL-safe base64url of canonical JSON:
  - `{"sessionKey":"...","sessionId":"..."}`
- Transcript detail routes must key on both `sessionKey` and `sessionId` semantically, but use the opaque `transcriptId` route token so raw session values do not become path-encoding hazards.
- Governance artifact pages read backend-owned distill artifacts:
  - `kind=lesson`
  - `kind=governance-candidate`
  - `subtype=follow-up-focus|next-turn-guidance`
- Behavioral Guidance remains a memory-row view over `category=behavioral`; it is not a distill-artifact list in disguise.

### List and pagination contract

- List routes must accept explicit paging inputs and return total-independent page cursors:
  - `limit`
  - `offset`
  - route-specific filters
- The first cut uses offset pagination with default `limit=50` and max `limit=200`, matching the backend’s current bounded list posture.
- Memory, distill, transcript, governance, and audit lists must define their stable sort order in DTO docs before implementation begins.

### Admin UX contract

- The admin UI should borrow the operator interaction model from `/root/code/Cli-Proxy-API-Management-Center`:
  - explicit login shell
  - persistent left navigation
  - sticky top status bar
  - quick-stat dashboard cards
  - secondary screen shells for detail/edit flows
  - confirmation and diff modals for destructive/config-changing actions
- This is a visual/interaction reference, not a branding clone.
- Chronicle Engine should keep its own naming and domain language while reusing the clearer management-center navigation and operator workflow patterns.
- Related page-level expectations:
  - Dashboard uses quick-stat cards as primary navigation accelerators.
  - Memories and Behavioral use dense tables plus secondary detail/edit shells.
  - Recall Lab uses an input/control region paired with a results/trace region.
  - Distill Jobs uses status-forward list rows and artifact detail shells.
  - Transcripts use a compact head list with chronological drill-down.
  - Governance uses explicit review/promote action surfaces with confirmation.
  - Settings uses structured sections plus diff-preview-before-apply.

## Event and Streaming Contracts

- No WebSocket or SSE requirement in this scope.
- Distill job status remains request/response polling based.
- The admin UI may poll principal-scoped distill endpoints for refresh:
  - `/admin/api/principals/{principalId}/distill/jobs`
  - `/admin/api/principals/{principalId}/distill/jobs/{jobId}`

## Error Model

- Admin auth failures must be clearly distinct from runtime auth failures.
- `/admin/api/*` must return admin-plane JSON errors, not HTML fallbacks.
- Unknown admin routes under `/admin/api/*` should return JSON `404`.
- `/admin` and `/admin/assets/*` must follow static asset semantics rather than runtime JSON errors.
- Admin auth failures should use `401`; rate limits should use `429`; unsupported principal/resource combinations should use `404` or `403` according to visibility rules.
- Admin memory mutations must preserve the current behavioral write restrictions:
  - no manual creation or update of backend-managed behavioral rows via ordinary memory-edit paths

## Validation and Compatibility Rules

- The backend remains the only authority; admin routes may orchestrate, aggregate, or reshape, but must not bypass storage/retrieval/distill logic.
- `auth.admin` becomes required in a meaningful way because it now guards the admin plane.
- `logging.level` becomes meaningful by driving backend logger setup.
- The logger implementation for this scope is `tracing` + `tracing-subscriber`, not ad hoc `println!` growth.
- The deployment model remains a single backend container with mounted LanceDB/SQLite/config volumes.
- The bundled admin UI must build into assets copied into the backend image; no second runtime container is introduced in this scope.
- `/admin` SPA fallback behavior must never intercept `/admin/api/*`; unknown API routes stay JSON `404`, while non-asset client routes under `/admin` resolve to the SPA shell.
