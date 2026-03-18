---
description: Single-contract scope for closing the remaining semantic authority gaps after turns-stage distill unification.
---

# distill-authority-closeout-2026-03-18 Contract

## Context

`turns-stage-distill-unification-2026-03-18` made backend-native distill the intended sole authority for trajectory-derived knowledge generation, but the current repo still contains a few semantic leftovers that can blur that boundary for maintainers and operators.

## Findings

- `docs/context-engine-split-2026-03-17/context-engine-split-contracts.md` still describes `command:new` / `command:reset` as a reflection-generation path even though current canonical docs and runtime code say that flow is removed.
- `backend/src/config.rs` and `deploy/backend.toml.example` still define a `providers.reflection` config block that no active backend runtime path consumes.
- Tool-facing memory category enums still include `reflection`, which keeps a manual write/update path open even though distill is meant to be the sole authority for trajectory-derived generation.
- Reflection recall/injection itself is still intentionally supported and must remain intact.

## Goals / Non-goals

### Goals
- Remove or demote top-level documentation that still presents reflection generation as current behavior.
- Remove unused backend reflection-provider config surface if it is truly dead.
- Tighten tool-facing/manual write paths if they currently let callers create new `reflection` rows in ways that contradict the intended authority boundary.
- Preserve supported reflection recall/injection runtime behavior.
- Leave the repo in a state where current top-level docs consistently say distill is the only trajectory-derived generation path.

### Non-goals
- Rework distill extraction semantics.
- Remove reflection recall/injection.
- Change the backend data model for existing reflection rows unless required for compile/test safety.
- Re-architect self-improvement tools.

## Target files / modules

- `docs/context-engine-split-2026-03-17/*`
- `docs/README.md`
- `docs/runtime-architecture.md`
- `backend/src/config.rs`
- `deploy/backend.toml.example`
- `src/backend-tools.ts`
- `src/backend-client/types.ts`
- tests covering config/schema/tool behavior and reflection recall compatibility

## Constraints

- Keep persistent docs in English.
- Preserve the current supported reflection recall/injection path.
- Do not touch global Codex state.
- Use the verify worktree only; do not modify the canonical `main` worktree directly.
- Prefer minimal, auditable changes over broad renames.

## Verification plan

- Run targeted repo scans for residual reflection-generation/current-state wording.
- Run targeted tests for config cutover, remote backend shell integration, backend contract semantics, and reflection recall behavior.
- Confirm no active top-level doc still presents command-triggered reflection generation as current.

## Rollback

- Discard the verify worktree branch `chore/distill-authority-closeout-2026-03-18` if the closeout proves too invasive.
- Revert only the changed files in this scope if a narrower rollback is needed.

## Open questions

- Should manual `memory_store` / `memory_update` calls be fully blocked from using `category=reflection`, or should that remain allowed for explicit operator-only backfill workflows?
- Should the old `context-engine-split-2026-03-17` snapshot stay top-level with stronger superseded labeling, or be moved under `docs/archive/`?

## Implementation evidence

- Updated top-level snapshot docs so `docs/context-engine-split-2026-03-17/*` no longer describe `command:new` / `command:reset` reflection generation as current behavior. The snapshot now points to `docs/runtime-architecture.md` and `docs/remote-memory-backend-2026-03-18/` as the active authority for that boundary, and it uses `session_end` / `before_reset` cleanup wording where the runtime still has active reflection-related hooks.
- Removed the dead backend reflection-provider config surface from `backend/src/config.rs` and `deploy/backend.toml.example`. Residual scan evidence: `rg -n "providers\\.reflection|ReflectionProviderConfig" backend/src deploy src test` returned no matches.
- Tightened public/manual write surfaces for reflection rows:
  - `src/backend-client/types.ts` now distinguishes `WritableMemoryCategory` from the broader read-side `MemoryCategory`.
  - `src/backend-tools.ts` keeps `reflection` available on read/debug surfaces, but `memory_store` and `memory_update` reject `category=reflection` locally with `code=reflection_category_reserved`.
  - `backend/src/models.rs` and `backend/src/state.rs` now reject manual reflection-row creation, retagging, and mutation with `INVALID_REQUEST`.
- Preserved active reflection recall/injection behavior by keeping read/debug/list shapes intact and by updating backend contract tests to seed reflection rows through a test-only direct table mutation helper instead of the now-blocked public write route.

## Verification evidence

- Top-level wording scan:
  - `rg -n "command:new|command:reset|reflection generation path|command-triggered reflection generation|/new|/reset" docs/context-engine-split-2026-03-17 docs/README.md docs/runtime-architecture.md README.md README_CN.md`
  - Result: only the canonical removal statement in `docs/runtime-architecture.md` and an explicit superseded-note line in `docs/context-engine-split-2026-03-17/context-engine-split-brainstorming.md`.
- Scope doc hygiene:
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/distill-authority-closeout-2026-03-18` -> `[OK] placeholder scan clean`
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/distill-authority-closeout-2026-03-18 README.md` -> passed
- JS test setup:
  - `npm ci` -> passed
- JS verification:
  - `node --test test/remote-backend-shell-integration.test.mjs --test-name-pattern='blocks manual reflection-category store/update writes before calling the backend|registers remote memory tools and forwards recall/store/forget/update without local scope authority payloads|uses backend reflection recall in before_prompt_build and preserves runtime context fields|removes command-triggered reflection generation hooks when no other command-hook feature is enabled'` -> passed, `23/23`
  - `node --test test/config-session-strategy-cutover.test.mjs` -> passed, `7/7`
  - `node --test test/memory-reflection.test.mjs --test-name-pattern='defaults auto-recall category allowlist to include other while keeping reflection excluded|plans reflection inherited-rules and error reminders via dedicated planner module|passes dynamic reflection candidate filters to backend recall|uses one shared session-clear contract for reflection errors and dynamic recall state'` -> passed, `29/29`
- Rust/backend verification:
  - `cargo test -q --test phase2_contract_semantics manual_reflection_write_routes_are_rejected` -> passed, `1 passed`
  - `cargo test -q --test phase2_contract_semantics debug_reflection_recall_route_reports_mode_and_trace_without_leaking_extra_row_fields` -> passed, `1 passed`
  - `cargo test -q --test phase2_contract_semantics reflection_recall_mode_honors_invariant_only_semantics` -> passed, `1 passed`
  - `cargo test -q --test phase2_contract_semantics generic_recall_applies_backend_owned_filter_fields` -> passed, `1 passed`
  - `cargo test -q --test phase2_contract_semantics reflection_recall_applies_include_kinds_filter` -> passed, `1 passed`

## Resolution notes

- Resolved: manual `memory_store` / `memory_update` calls are now fully blocked from creating, retagging, or mutating `reflection` rows on supported public write surfaces.
- Resolved: `docs/context-engine-split-2026-03-17/` stayed top-level, but now carries stronger superseded labeling instead of parallel-current wording about reflection generation.
