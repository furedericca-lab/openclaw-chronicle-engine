Phase 2 closeout continuation for remote-memory-backend in this worktree.

Repo: /root/verify/memory-lancedb-pro-context-engine-split
Branch: dev/context-engine-split
Previous Codex runs:
- 20260313T025626Z-memory-lancedb-pro-context-engine-split-write
- 20260313T034425Z-memory-lancedb-pro-context-engine-split-write

Read first:
- docs/remote-memory-backend/remote-memory-backend-contracts.md
- docs/remote-memory-backend/technical-documentation.md
- docs/remote-memory-backend/remote-memory-backend-scope-milestones.md
- docs/remote-memory-backend/task-plans/phase-2-remote-memory-backend.md
- docs/remote-memory-backend/task-plans/4phases-checklist.md
- docs/remote-memory-backend/task-plans/phase-2-implementation-handoff.md
- docs/remote-memory-backend/task-plans/codex-phase-2-task.md
- docs/remote-memory-backend/task-plans/codex-phase-2-round2-task.md

This is a continuation. Do not repeat finished work. Continue from actual repo state.

Reviewer closeout findings on the current backend implementation:

1. BLOCKER — implementation/auth contract drift: runtime identity now depends on `x-auth-user-id` and `x-auth-agent-id`, but the frozen contract/docs do not define this trusted identity handoff.
   Evidence:
   - backend/src/lib.rs:30-31, 218-237
   - tests now auto-infer these headers from request bodies, which masks the real requirement.
   Required fix direction:
   - choose one implementation-consistent closeout path and make code/tests/docs agree:
     a) document these as trusted gateway-injected runtime identity headers for MVP and harden tests around missing/forged header behavior; or
     b) replace them with a different formal auth-context derivation that matches the docs better.
   Constraint:
   - do not silently leave an undocumented auth boundary in place.

2. IMPORTANT — idempotency reserve happens before the side effect and burns the key even if the downstream write fails; replay semantics are still incomplete.
   Evidence:
   - backend/src/lib.rs:98-115, 118-135, 138-155, 182-202
   - backend/src/state.rs:489-559
   Required fix direction:
   - move toward a stateful idempotency record that can distinguish reserved/in-progress/completed/failed;
   - avoid permanently consuming a key when the protected operation fails before commit;
   - if full response replay is still too large for this round, at minimum make failure recovery semantics explicit and safe.

3. IMPORTANT — memory update is delete-then-insert and is not atomic, so a failed insert can lose the row.
   Evidence:
   - backend/src/state.rs:136-165
   Required fix direction:
   - make update safer/atomic enough for MVP, or explicitly stage/verify insert before destructive delete if LanceDB API forces a replace pattern.

4. IMPORTANT — tests/helper currently auto-populate auth identity headers from the actor body, which can hide real caller-context requirements.
   Evidence:
   - backend/tests/phase2_contract_semantics.rs: request_json helper
   Required fix direction:
   - make tests explicit about when trusted auth headers are provided vs omitted;
   - add at least one negative test for missing authenticated identity context.

Goal for this continuation:
- close the highest-risk Phase 2 gaps so Phase 2 can be judged near-complete rather than just partial.

Preferred priorities:
1. Resolve auth-boundary contract drift (implementation + docs + tests must agree)
2. Improve idempotency failure semantics beyond one-shot reservation burn
3. Make update path safer than delete-then-insert loss risk
4. Update checklist/docs to reflect the actual closeout state

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
- add/update tests that prove the chosen trusted auth-context contract explicitly;
- add/update tests covering idempotency failure/closeout semantics if you change them;
- if docs change, keep contracts/technical-docs/checklist consistent.

If meaningful code lands, update:
- docs/remote-memory-backend/remote-memory-backend-contracts.md
- docs/remote-memory-backend/technical-documentation.md
- docs/remote-memory-backend/task-plans/4phases-checklist.md

Preferred report:
- status
- changed files
- which closeout findings were fixed
- verification results
- remaining blockers for Phase 2 sign-off if any
