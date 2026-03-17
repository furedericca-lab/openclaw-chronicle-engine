Phase 2 continuation for remote-memory-backend in this worktree.

Repo: /root/verify/memory-lancedb-pro-context-engine-split
Branch: dev/context-engine-split
Previous Codex run: 20260313T025626Z-memory-lancedb-pro-context-engine-split-write

Read first:
- docs/remote-memory-backend/remote-memory-backend-contracts.md
- docs/remote-memory-backend-2026-03-17/remote-memory-backend-2026-03-17-technical-documentation.md
- docs/remote-memory-backend/remote-memory-backend-scope-milestones.md
- docs/remote-memory-backend/task-plans/phase-2-remote-memory-backend.md
- docs/remote-memory-backend/task-plans/4phases-checklist.md
- docs/remote-memory-backend/task-plans/phase-2-implementation-handoff.md
- docs/remote-memory-backend/task-plans/codex-phase-2-task.md

This is a continuation. Do not repeat finished work. Continue from actual repo state.

Reviewer findings on the first batch (treat these as must-address in this continuation):

1. BLOCKER — caller principal for `GET /v1/reflection/jobs/{jobId}` is still taken from ad hoc headers (`x-actor-user-id`, `x-actor-agent-id`) instead of the same authenticated runtime context used by the rest of the data plane.
   Evidence:
   - backend/src/lib.rs:get_reflection_job_status reads those headers directly.
   - backend/tests/phase2_contract_semantics.rs verifies that header path instead of formal auth/context wiring.
   Required fix direction:
   - introduce a formal authenticated request context / actor-principal extraction path;
   - remove the ad hoc caller-principal headers from the data-plane job-status route;
   - update tests so ownership is proven through the formal request context, not special review-only headers.

2. IMPORTANT — runtime auth currently validates only the bearer token, but does not bind the request actor to authenticated request context.
   Evidence:
   - backend/src/lib.rs:runtime_auth_middleware only checks `Authorization` and `x-request-id`.
   - handlers still trust caller-supplied actor bodies directly for store/update/delete/list/stats/recall/job enqueue.
   Required fix direction:
   - create a request-auth context that carries the caller principal used by handlers;
   - explicitly decide and implement how the actor envelope is validated against that context in MVP;
   - do not leave data-plane authorization as an unbound bearer token plus arbitrary actor body forever.

3. IMPORTANT — memory persistence is still an in-memory scaffold, so T102 is not really complete.
   Evidence:
   - backend/src/state.rs uses `MemoryRepo { rows: HashMap<...> }` only.
   Required fix direction:
   - integrate LanceDB-backed persistence for the memory path, or at minimum land the real storage seam and first functional LanceDB-backed store/list/stats/recall path.

4. IMPORTANT — reflection recall currently ignores `mode` semantics and always emits derived rows with placeholder values.
   Evidence:
   - backend/src/state.rs:recall_reflection ignores `req.mode` and always returns `ReflectionKind::Derived` with `strict_key: None`.
   Required fix direction:
   - either implement the minimum meaningful `mode` behavior now, or explicitly constrain the scaffold with tests/comments/docs so the remaining contract gap is visible and auditable.

5. IMPORTANT — idempotency is header-presence only; conflict semantics are not implemented.
   Evidence:
   - backend/src/lib.rs only requires `idempotency-key` presence.
   Required fix direction:
   - if full idempotency storage is too much for this continuation, at least leave a clear storage seam / tracked follow-up contract note and avoid implying the route is fully compliant.

Primary implementation goals for this continuation:
- replace the in-memory memory path with LanceDB-backed persistence or a credible real storage seam that actually exercises LanceDB;
- move caller principal derivation away from special headers and into formal auth/context wiring shared by the data plane;
- tighten tests to prove the new ownership/auth path;
- keep frozen contract points unchanged.

Frozen points you must preserve:
- POST /v1/memories/store supports exactly two request shapes via mode: tool-store and auto-capture.
- tool-store preserves explicit category and importance.
- scope is forbidden in ordinary runtime write/update payloads.
- POST /v1/memories/update exists as a dedicated endpoint.
- POST /v1/memories/stats remains the canonical data-plane stats route.
- Reflection job status on the data plane is caller-scoped by `(userId, agentId)`.
- Operator-global inspection belongs only to admin routes.
- POST /v1/memories/list uses frozen category enum, default createdAt DESC ordering, and nextOffset: null on the final page.
- sessionKey is stable logical provenance; sessionId is ephemeral diagnostics only.
- Stable recall DTOs must not expose raw vector/BM25/rerank breakdown internals.
- Do not expand into deferred CLI/operator surfaces.

Verification expectations:
- run backend tests after changes;
- add/update tests that prove job-status ownership without ad hoc caller headers;
- if LanceDB integration lands, include at least one storage-backed contract test or check proving the code is no longer purely in-memory.

If meaningful code lands, update:
- docs/remote-memory-backend/task-plans/4phases-checklist.md

Preferred report:
- status
- changed files
- which reviewer findings were fixed
- verification results
- remaining blockers for the next continuation if not done
