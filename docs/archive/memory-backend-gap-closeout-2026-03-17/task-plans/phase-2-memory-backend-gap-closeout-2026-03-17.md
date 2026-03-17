---
description: Task list for memory-backend-gap-closeout-2026-03-17 phase 2.
---

# Tasks: memory-backend-gap-closeout-2026-03-17 Phase 2

## Input
- Canonical sources:
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/memory-backend-gap-closeout-2026-03-17/memory-backend-gap-closeout-2026-03-17-scope-milestones.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/memory-backend-gap-closeout-2026-03-17/memory-backend-gap-closeout-2026-03-17-technical-documentation.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/memory-backend-gap-closeout-2026-03-17/memory-backend-gap-closeout-2026-03-17-contracts.md

## Canonical architecture / Key constraints
- runtime reflection flow must no longer depend on local session file recovery;
- status surface stays caller-scoped and management-gated;
- no local fallback or scope override may appear.

## Phase 2: Reflection Runtime Closeout
Goal: implement the backend-owned reflection source path and expose reflection status through the plugin surface.

Definition of Done: reflection enqueue and reflection status behavior are implemented, test-backed, and principal-safe.

Tasks:
- [x] T021 [Backend] Implement or finalize the backend-owned reflection source path needed by `/new` and `/reset`.
  - DoD: backend route or state-layer support exists and is covered by backend tests.
- [x] T022 [P] [Agentic] Rewire `index.ts` reflection hooks and `src/backend-client/*` to stop using local session-file recovery in supported runtime.
  - DoD: supported runtime path no longer depends on `readSessionConversationWithResetFallback()` or `resolveReflectionSessionSearchDirs()`.
- [x] T023 [P] [Agentic] Add `memory_reflection_status` to `src/backend-tools.ts` and wire it to typed backend-client status calls.
  - DoD: management gating, fail-closed behavior, and structured tool output are implemented and test-backed.
- [x] T024 [QA] Add/update backend and Node integration coverage for:
  - transcript-backed reflection source behavior;
  - reflection status success path;
  - missing-principal and cross-principal negative paths.
  - DoD: relevant test commands pass or environment-blocked status is documented.

Checkpoint: reflection runtime authority and status visibility are closed out without local session-file dependence.

## Evidence

- Backend route and state support landed in `backend/src/lib.rs`, `backend/src/models.rs`, and `backend/src/state.rs`.
- Runtime hook rewiring landed in `index.ts`, `src/backend-client/client.ts`, and `src/backend-client/types.ts`.
- Tool exposure landed in `src/backend-tools.ts`.
- Backend contract test coverage landed in `backend/tests/phase2_contract_semantics.rs`.
- Node integration coverage landed in `test/remote-backend-shell-integration.test.mjs`.

## Verification Commands

- `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
- `npm test`
- `rg -n "readSessionConversationWithResetFallback|resolveReflectionSessionSearchDirs|session-recovery\\.ts" index.ts src test`

## Dependencies & Execution Order
- Depends on Phase 1.
- T021 should land before or together with T022.
- T023 and T024 may proceed in parallel when file ownership does not overlap.
