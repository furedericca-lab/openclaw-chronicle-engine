---
description: Execution and verification checklist for the remote-memory-backend 4-phase plan.
---

# Phases Checklist: remote-memory-backend

## Input

- `docs/remote-memory-backend/remote-memory-backend-brainstorming.md`
- `docs/remote-memory-backend/remote-memory-backend-implementation-research-notes.md`
- `docs/remote-memory-backend/remote-memory-backend-scope-milestones.md`
- `docs/remote-memory-backend/technical-documentation.md`
- `docs/remote-memory-backend/remote-memory-backend-contracts.md`
- `docs/remote-memory-backend/phase-2-sign-off-note.md`
- `docs/remote-memory-backend/task-plans/phase-1-remote-memory-backend.md`
- `docs/remote-memory-backend/task-plans/phase-2-remote-memory-backend.md`
- `docs/remote-memory-backend/task-plans/phase-3-remote-memory-backend.md`
- `docs/remote-memory-backend/task-plans/phase-4-remote-memory-backend.md`

## Global Status Board

| Phase | Status | Completion | Health | Blockers |
|---|---|---|---|---|
| 1 | Planned | 100% docs | Green | 0 |
| 2 | Accepted for gated transition | 65% | Yellow | 2 |
| 3 | Closeout completed (shell remote-path evidence landed) | Preconditions 100% / Integration 100% | Green | 0 |
| 4 | Verification and release-readiness closeout completed | 100% | Green | 0 |

## Phase Entry Links

1. [phase-1-remote-memory-backend.md](./phase-1-remote-memory-backend.md)
2. [phase-2-remote-memory-backend.md](./phase-2-remote-memory-backend.md)
3. [phase-3-remote-memory-backend.md](./phase-3-remote-memory-backend.md)
4. [phase-4-remote-memory-backend.md](./phase-4-remote-memory-backend.md)

## Phase Execution Records

### Phase 1

- Status: Planned / docs completed
- Batch date: 2026-03-13
- Completed tasks:
  - Updated the frozen contract set in `docs/remote-memory-backend/remote-memory-backend-contracts.md`.
  - Froze `POST /v1/memories/store` into two request shapes:
    - `mode=tool-store` for explicit tool writes with `category` / `importance`
    - `mode=auto-capture` for transcript ingest
  - Added a dedicated `POST /v1/memories/update` endpoint to preserve explicit update semantics in MVP.
  - Replaced the data-plane stats route shape with `POST /v1/memories/stats` to keep actor-envelope rules consistent.
  - Froze reflection job ownership and visibility:
    - data-plane visibility is caller-scoped by `(userId, agentId)` principal
    - operator-global inspection belongs to admin routes only
  - Froze list semantics:
    - category uses a frozen enum set
    - default ordering is `createdAt DESC`
    - `nextOffset` is `null` on the final page
  - Clarified actor semantics:
    - `sessionKey` = stable logical session identity
    - `sessionId` = ephemeral diagnostics identity
  - Narrowed stable recall DTOs to orchestration-facing fields and explicitly excluded raw scoring-breakdown internals from the stable runtime contract.
  - Documented the MVP remote parity boundary vs deferred CLI/operator capabilities in the technical docs and milestones.
  - Expanded phase task plans so phases 2-4 explicitly test and preserve the newly frozen semantics.
- Evidence commands:
  - `find docs/remote-memory-backend -maxdepth 2 -type f | sort`
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/remote-memory-backend`
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/remote-memory-backend README.md`
  - `rg -n "POST /v1/memories/store|POST /v1/memories/update|POST /v1/memories/stats|GET /v1/reflection/jobs/\{jobId\}|createdAt DESC|sessionKey|sessionId|MVP runtime parity boundary" docs/remote-memory-backend`
- Issues/blockers:
  - The earlier docs left several contract points under-specified for implementation handoff, especially store/update/stats/job-status semantics.
- Resolutions:
  - Hardened the docs and task plans before backend implementation starts so Phase 2 can proceed from a tighter freeze.
- Checkpoint confirmed:
  - Yes. Phase 2 may start from the frozen authority and API contract.

### Phase 2

- Status: Accepted for gated transition / sign-off note recorded
- Batch date: 2026-03-13
- Completed tasks:
  - Tightened the auth-boundary contract across implementation, tests, and docs:
    - runtime principal headers (`x-auth-user-id`, `x-auth-agent-id`) remain the trusted request context in middleware;
    - actor envelopes are now explicitly required to match that trusted context;
    - docs define these headers as gateway/runtime-auth injected identity handoff for MVP.
  - Reworked idempotency handling from one-shot reservation into explicit lifecycle state transitions in `backend/src/state.rs`:
    - reservation now records `reserved -> in_progress -> completed|failed`;
    - failed protected operations no longer permanently burn keys;
    - same-key retry after failed operations is allowed only for matching payload fingerprints.
  - Removed delete-then-insert risk from memory updates.
  - Recorded `docs/remote-memory-backend/phase-2-sign-off-note.md` to accept Phase 2 for gated transition while preserving explicit deferreds and verification blockers.
- Evidence commands:
  - `cd backend && cargo fmt`
  - `cd backend && cargo check --all-targets`
  - `cd backend && cargo test --all-targets` (verification path still unstable across environments)
  - `rg -n "run_idempotent_operation|x-auth-user-id|x-auth-agent-id|ensure_actor_matches_context" backend/src/lib.rs`
  - `rg -n "IdempotencyStatus|mark_failed|mark_completed|update semantics|LanceDB" backend/src/state.rs docs/remote-memory-backend/*.md`
- Issues/blockers:
  - Full idempotent response replay for completed requests remains deferred; completed-key repeats still return `409 IDEMPOTENCY_CONFLICT`.
  - A clean, stable full verification stamp has not yet been obtained due to linker / resource / outer-run instability.
- Resolutions:
  - Phase 2 was signed off for **gated transition** instead of fully closed completion.
  - Verification blockers are now carried explicitly into the Phase 3 precondition gate rather than being hidden.
- Checkpoint confirmed:
  - Yes, for gated transition only. Phase 3 planning may proceed, but Phase 3 main integration work is blocked on the precondition verification batch.

### Phase 3

- Status: Main integration and closeout evidence completed after precondition gate
- Batch date: 2026-03-13
- Completed tasks:
  - `T3P01` completed with a stable verification path in `backend/`:
    - used `~/.cargo/bin/cargo` (`cargo 1.94.0`) instead of system `/usr/bin/cargo` (`cargo 1.65.0`) because lockfile v4 is unsupported on the system cargo.
    - used serialized build jobs (`CARGO_BUILD_JOBS=1`, `-j 1`) to reduce linker/resource spikes.
  - `T3P02` completed with targeted high-risk backend semantic tests:
    - auth-context binding:
      - `actor_principal_must_match_authenticated_request_context`
      - `missing_authenticated_identity_headers_are_rejected`
    - LanceDB persistence:
      - `lancedb_memory_persists_across_app_restart`
    - idempotency lifecycle:
      - `idempotency_reuse_returns_conflict`
      - `idempotency_key_can_retry_after_failed_operation`
    - reflection job ownership:
      - `reflection_job_status_is_scoped_to_user_and_agent`
  - `T3P03` completed:
    - no outer-run `SIGTERM` interruption occurred on the stable path;
    - the longest run in this batch was the first targeted test build (`8m 12s`), which completed normally.
  - `T3P04` completed:
    - checklist now records exact commands, exact outcomes, and the gate decision.
  - `T201` completed:
    - introduced a dedicated shell adapter seam under `src/backend-client/`:
      - `client.ts` for base URL/token loading, header shaping, retry/error translation, and endpoint methods
      - `runtime-context.ts` for actor-envelope + trusted runtime identity extraction (`sessionKey` + `sessionId` preserved)
      - `types.ts` for frozen route DTOs used by shell-side rewiring
    - `index.ts` now builds and wires this client when `remoteBackend` config is enabled.
  - `T202` completed:
    - `src/context/auto-recall-orchestrator.ts` now accepts backend recall rows through adapter-facing dependencies.
    - `src/context/reflection-prompt-planner.ts` now supports adapter-backed reflection recall (`invariant-only` / `invariant+derived`) while keeping local session-state orchestration.
    - `index.ts` rewired planner dependencies to backend client paths in remote mode.
  - `T203` completed:
    - added `src/backend-tools.ts` and rewired runtime memory tools (`memory_recall`, `memory_store`, `memory_forget`, `memory_update`, optional list/stats) to backend routes.
    - `agent_end` auto-capture now forwards transcript items to `POST /v1/memories/store` with `mode=auto-capture` in remote mode.
  - `T204` completed:
    - `command:new` / `command:reset` reflection path now enqueues backend jobs asynchronously (`POST /v1/reflection/jobs`) and returns without waiting for reflection completion.
  - `T205` completed:
    - adapter context builder keeps actor principal and trusted identity header forwarding aligned.
    - `sessionKey` stays the stable logical provenance field; `sessionId` remains runtime-ephemeral diagnostics input.
    - scope/ACL authority is not recomputed in shell adapter paths.
  - Phase 3 closeout evidence completed:
    - new focused remote integration suite added: `test/remote-backend-shell-integration.test.mjs`.
    - this suite directly verifies remote tools, auto-capture forwarding, reflection recall at `before_prompt_build`, async `/new` + `/reset` enqueue, and runtime context propagation (`userId`, `agentId`, `sessionId`, `sessionKey`).
    - assertions explicitly confirm remote-mode payloads do not include local scope authority fields (`scope` / `scopeFilter`), preventing mixed-authority regressions.
- Evidence commands:
  - `cd backend && CARGO_BUILD_JOBS=1 cargo check --all-targets -j 1` (failed on system cargo 1.65 due to lockfile v4)
  - `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable`
  - `cd backend && ~/.cargo/bin/cargo --version && ~/.cargo/bin/rustc --version`
  - `cd backend && CARGO_BUILD_JOBS=1 ~/.cargo/bin/cargo check --all-targets -j 1 --locked`
  - `cd backend && CARGO_BUILD_JOBS=1 ~/.cargo/bin/cargo test --locked --test phase2_contract_semantics actor_principal_must_match_authenticated_request_context -- --exact`
  - `cd backend && CARGO_BUILD_JOBS=1 ~/.cargo/bin/cargo test --locked --test phase2_contract_semantics missing_authenticated_identity_headers_are_rejected -- --exact`
  - `cd backend && CARGO_BUILD_JOBS=1 ~/.cargo/bin/cargo test --locked --test phase2_contract_semantics lancedb_memory_persists_across_app_restart -- --exact`
  - `cd backend && CARGO_BUILD_JOBS=1 ~/.cargo/bin/cargo test --locked --test phase2_contract_semantics idempotency_reuse_returns_conflict -- --exact`
  - `cd backend && CARGO_BUILD_JOBS=1 ~/.cargo/bin/cargo test --locked --test phase2_contract_semantics idempotency_key_can_retry_after_failed_operation -- --exact`
  - `cd backend && CARGO_BUILD_JOBS=1 ~/.cargo/bin/cargo test --locked --test phase2_contract_semantics reflection_job_status_is_scoped_to_user_and_agent -- --exact`
  - `node --test --test-name-pattern='.' test/remote-backend-shell-integration.test.mjs`
  - `npm test`
  - `git diff -- index.ts src/backend-client/client.ts src/backend-client/runtime-context.ts src/backend-client/types.ts src/backend-tools.ts src/context/auto-recall-orchestrator.ts src/context/reflection-prompt-planner.ts openclaw.plugin.json`
- Issues/blockers:
  - Initial environment blocker resolved in-batch:
    - system `/usr/bin/cargo` cannot parse `Cargo.lock` version 4.
  - Remaining environment note (not a blocker):
    - backend verification in this environment must use `~/.cargo/bin/cargo` (rustup toolchain) rather than system cargo.
  - Residual integration risk:
    - remote identity fallback values (`remoteBackend.userIdFallback` / `agentIdFallback`) should remain a last-resort path; production should provide trusted runtime identity fields.
- Resolutions:
  - Credible verification stamp obtained via stable command path:
    - `cargo check` passed;
    - all required targeted semantic tests passed;
    - no outer `SIGTERM` recurrence on this execution path.
  - Phase 3 shell integration seam and main rewiring landed behind explicit `remoteBackend` enablement config.
  - Phase 3 closeout blocker (missing remote-path shell evidence) is cleared by the focused remote integration test suite plus full `npm test` pass.
- Checkpoint confirmed:
  - Yes. Preconditions remain satisfied, T201-T205 are completed, and Phase 3 is ready to hand off to Phase 4 verification/e2e work.

### Phase 4

- Status: Completed
- Batch date: 2026-03-13
- Completed tasks:
  - `T301` completed:
    - shell negative-path coverage added in `test/remote-backend-shell-integration.test.mjs` for:
      - fail-open generic recall on backend failure,
      - fail-open reflection recall on backend failure,
      - surfaced write/update/delete failures in remote mode,
      - non-blocking reflection enqueue plus failure observability.
  - `T304` completed:
    - contract-hardening edge verification is now explicit in `backend/tests/phase2_contract_semantics.rs`:
      - caller-scoped reflection job visibility,
      - actor-envelope stats behavior,
      - list ordering + terminal `nextOffset=null`,
      - forbidden scope and frozen-category rejection.
  - `T303` completed:
    - admin-token isolation coverage added in `backend/tests/phase2_contract_semantics.rs`:
      - admin token cannot bypass data-plane auth,
      - unauthorized responses do not leak token values,
      - `/v1/admin/*` remains non-runtime-accessible in current MVP implementation.
  - `T302` + `T305` completed:
    - migration cutover/rollback runbook and deferred parity boundary finalized in:
      - `docs/remote-memory-backend/technical-documentation.md`
      - `docs/remote-memory-backend/phase-4-verification-status-report.md`
- Evidence commands:
  - `node --test --test-name-pattern='.' test/remote-backend-shell-integration.test.mjs`
  - `cd backend && CARGO_BUILD_JOBS=1 ~/.cargo/bin/cargo test --locked --test phase2_contract_semantics -- --nocapture`
  - `cd backend && CARGO_BUILD_JOBS=1 ~/.cargo/bin/cargo check --all-targets -j 1 --locked`
  - `npm test`
- Issues/blockers:
  - No blocking defects found in this phase-closeout batch.
- Resolutions:
  - Phase 4 closeout now provides direct test and documentation evidence for all planned tasks.
- Checkpoint confirmed:
  - Yes.

## Final release gate

- [x] Authority model documented with singular backend ownership.
- [x] Phased docs created for implementation and auditability.
- [x] Contract and technical docs include rollback and failure behavior.
- [x] Store/update/stats/job-visibility/list semantics hardened for implementation handoff.
- [x] MVP parity boundary vs deferred CLI/operator surface is explicit.
- [x] Phase 2 sign-off note recorded for gated transition.
- [x] Backend implementation completed.
- [x] Phase 3 precondition verification stamp recorded.
- [x] Shell integration completed.
- [x] End-to-end verification completed.
