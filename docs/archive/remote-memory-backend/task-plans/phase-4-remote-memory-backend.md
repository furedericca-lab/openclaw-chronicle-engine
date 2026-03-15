---
description: Verification and operator-readiness tasks for remote-memory-backend.
---

# Tasks: remote-memory-backend

## Input

- `docs/remote-memory-backend/remote-memory-backend-contracts.md`
- `docs/remote-memory-backend/technical-documentation.md`
- phases 2-3 implementation results
- local shell tests and backend contract tests

## Canonical architecture / Key constraints

- No mixed-authority rollback shortcuts.
- Recall remains fail-open.
- Explicit write/update/delete failures remain surfaced.
- Reflection enqueue remains non-blocking.
- Admin endpoints are a separate audited control plane; admin-token bypass applies only there.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 4: Verification, migration safety, and ops readiness
Goal: validate behavior, migration safety, and operator-facing guardrails for the remote backend split.
Definition of Done: shell and backend tests prove the new authority model, failure behavior, and async reflection semantics are working and documented.

Tasks:
- [x] T301 [QA] Run backend contract tests and shell behavior tests against the agreed failure semantics.
  - DoD: evidence includes passing tests for fail-open recall, surfaced write/update/delete errors, and non-blocking reflection enqueue.
- [x] T302 [P] [Docs] Finalize migration and rollback notes for singular authority.
  - DoD: docs explain how to cut over fully to remote authority and how to revert fully without mixed fallback behavior.
- [x] T303 [P] [Security] Verify admin-token bypass is isolated to explicit management routes and does not leak secrets in responses.
  - DoD: operator/admin paths are documented and test-covered with clear control-plane vs data-plane separation and auditable token-class handling.
- [x] T304 [P] [QA] Verify contract-hardening edge cases introduced by the freeze.
  - DoD: evidence covers caller-scoped reflection job visibility, actor-envelope stats behavior, list ordering/terminal pagination, and rejection of non-frozen category or forbidden scope payloads.
- [x] T305 [Docs] Confirm MVP parity boundaries and deferred surfaces in release-facing docs.
  - DoD: docs clearly state which local CLI/operator features remain deferred so MVP sign-off is not blocked by parity creep.

## Phase 4 readiness baseline (2026-03-13 after Phase 3 closeout)

- Completed before Phase 4 start:
  - focused remote shell integration suite now exists at `test/remote-backend-shell-integration.test.mjs`
  - suite verifies remote tool routing, auto-capture forwarding, reflection recall in `before_prompt_build`, and non-blocking enqueue for `/new` + `/reset`
  - suite verifies runtime context propagation and confirms no local scope authority payload fields in remote mode
  - repo verification baseline: `npm test` passed with this suite included
- Still required in Phase 4:
  - backend+shell cross-surface failure-semantic evidence required by `T301` (`fail-open` recall path and surfaced write/update/delete failures under negative cases)
  - operator/admin isolation verification and contract-edge hardening (`T303`, `T304`)
  - migration/rollback and parity/deferred-surface finalization docs (`T302`, `T305`)

Checkpoint: the migration is reviewable and reversible without ambiguous authority or hidden fallback paths.

## Phase 4 execution record (2026-03-13)

- `T301` completed with explicit negative-path shell coverage in `test/remote-backend-shell-integration.test.mjs`:
  - generic recall fail-open in `before_agent_start` when `/v1/recall/generic` fails;
  - reflection recall fail-open in `before_prompt_build` when `/v1/recall/reflection` fails;
  - surfaced remote write/update/delete failures in `memory_store` / `memory_update` / `memory_forget`;
  - non-blocking reflection enqueue verified for both success and failure-observability paths.
- `T304` completed with backend contract-edge verification in `backend/tests/phase2_contract_semantics.rs`:
  - caller-scoped reflection job visibility (`reflection_job_status_is_scoped_to_user_and_agent`);
  - stats actor-envelope principal enforcement (`stats_actor_envelope_must_match_authenticated_principal`);
  - list default ordering + terminal `nextOffset=null` semantics (`list_default_order_and_final_page_next_offset_null`);
  - forbidden scope and frozen category rejection (`write_payloads_forbid_scope_fields`, `frozen_category_enum_is_enforced`).
- `T303` completed via explicit token-boundary tests in `backend/tests/phase2_contract_semantics.rs`:
  - `admin_token_cannot_bypass_data_plane_and_admin_routes_are_not_exposed` verifies:
    - admin token cannot access data-plane write route;
    - unauthorized error payload does not leak token material;
    - `/v1/admin/*` remains non-runtime-accessible in current MVP implementation.
- `T302` + `T305` completed by finalizing migration/rollback runbook and deferred parity boundaries in:
  - `docs/remote-memory-backend/technical-documentation.md`
  - `docs/remote-memory-backend/phase-4-verification-status-report.md`

Evidence commands (2026-03-13):

- `node --test --test-name-pattern='.' test/remote-backend-shell-integration.test.mjs`
  - result: pass (`10/10`)
- `cd backend && CARGO_BUILD_JOBS=1 ~/.cargo/bin/cargo test --locked --test phase2_contract_semantics -- --nocapture`
  - result: pass (`16/16`)
- `cd backend && CARGO_BUILD_JOBS=1 ~/.cargo/bin/cargo check --all-targets -j 1 --locked`
  - result: pass
- `npm test`
  - result: pass (`157/157` + CLI smoke)

Phase 4 checkpoint: completed.

## Dependencies & Execution Order

- Phase 4 depends on Phases 2-3.
- Tasks marked `[P]` may run concurrently only if they do not touch the same files.
