Phase 2 start for remote-memory-backend in this worktree.

Repo: /root/verify/memory-lancedb-pro-context-engine-split
Branch: dev/context-engine-split

Read and follow these canonical docs first:
- docs/remote-memory-backend/remote-memory-backend-contracts.md
- docs/remote-memory-backend-2026-03-17/remote-memory-backend-2026-03-17-technical-documentation.md
- docs/remote-memory-backend/remote-memory-backend-scope-milestones.md
- docs/remote-memory-backend/task-plans/phase-2-remote-memory-backend.md
- docs/remote-memory-backend/task-plans/4phases-checklist.md
- docs/remote-memory-backend/task-plans/phase-2-implementation-handoff.md

Task:
- Investigate current repo state and identify the concrete backend implementation starting point for Phase 2.
- Implement the smallest credible first batch toward T101-T105.
- Preserve all frozen contract points; do not reopen them unless code reveals a hard contradiction.
- Add or update tests that lock the frozen runtime semantics where practical.
- If meaningful code lands, update docs/remote-memory-backend/task-plans/4phases-checklist.md with Phase 2 evidence.

Frozen points you must obey:
- POST /v1/memories/store supports exactly two request shapes via mode: tool-store and auto-capture.
- tool-store preserves explicit category and importance.
- scope is forbidden in ordinary runtime write/update payloads.
- POST /v1/memories/update exists as a dedicated endpoint.
- Data-plane stats route is POST /v1/memories/stats.
- Reflection job status on the data plane is caller-scoped by (userId, agentId).
- Operator-global inspection belongs only to admin routes.
- POST /v1/memories/list uses frozen category enum, default createdAt DESC ordering, and nextOffset: null on the final page.
- sessionKey is stable logical provenance; sessionId is ephemeral diagnostics only.
- Stable recall DTOs must not expose raw vector/BM25/rerank breakdown internals.
- Remote MVP parity is intentionally limited; do not expand into deferred CLI/operator surfaces.

Preferred output:
- status
- changed files
- implemented endpoints/scaffolds
- tests added/updated
- verification results
- blockers / next continuation requirement if unfinished
