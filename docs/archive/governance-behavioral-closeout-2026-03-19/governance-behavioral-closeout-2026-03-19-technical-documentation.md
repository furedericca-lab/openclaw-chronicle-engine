---
description: Canonical technical architecture for governance-behavioral-closeout-2026-03-19.
---

# governance-behavioral-closeout-2026-03-19 Technical Documentation

## Canonical Architecture

- Governance is the only active backlog/review/promotion workflow surface.
- Behavioral guidance is the only active plugin/runtime concept for prompt-time inherited/adaptive reminder injection.
- Distill remains the only trajectory-derived generation/write authority.
- The adapter layer maps behavioral-guidance terminology onto the stable backend reflection routes instead of preserving user-facing compatibility aliases.

## Key Constraints and Non-Goals

- Non-goal: backend wire/storage rename.
- Non-goal: restore `.learnings/` compatibility or `self_improvement_*` aliases.
- Non-goal: introduce a migration note or release-note style compatibility chapter.

## Module Boundaries and Data Flow

- Governance surface:
  - `src/governance-tools.ts`
  - `test/governance-tools.test.mjs`
- Behavioral-guidance prompt orchestration:
  - `src/context/auto-recall-orchestrator.ts`
  - `src/context/session-exposure-state.ts`
  - `src/context/behavioral-guidance-error-signals.ts`
  - `src/context/prompt-block-renderer.ts`
  - `test/helpers/behavioral-guidance-reference.ts`
- Adapter/backend-client surface:
  - `src/backend-client/types.ts`
  - `src/backend-client/client.ts`
  - `src/backend-tools.ts`
  - `index.ts`
- Backend internal helper closeout:
  - `backend/src/lib.rs`
  - `backend/src/models.rs`
  - `backend/src/state.rs`
- Archived docs disposition:
  - `docs/archive/autorecall-governance-unification-2026-03-18/`

## Interfaces and Contracts

- Governance tool registration:
  - only `governance_log`, `governance_review`, `governance_extract_skill`
- Debug recall management tool:
  - `channel="generic" | "behavioral"`
  - `behavioralMode` for the behavioral lane
  - backend route remains `/v1/debug/recall/reflection`
- Config parsing:
  - canonical fields only;
  - removed aliases are rejected explicitly.
- Backend naming boundary:
  - active internal helper names now prefer behavioral-guidance wording;
  - route/storage/data fields still use `reflection`.

## Security and Reliability

- Manual writes to `category=reflection` are still blocked at both plugin and backend layers.
- Recall paths remain fail-open where already intended:
  - generic autoRecall
  - behavioral-guidance recall
- Write/update/delete/distill/debug management paths remain fail-closed on missing runtime principal identity.
- No compatibility wrapper remains to silently normalize legacy config/tool/module names.

## Test Strategy

- JS suite:
  - `npm ci`
  - `node --test --test-name-pattern='.' test/governance-tools.test.mjs test/auto-recall-behavioral.test.mjs test/config-session-strategy-cutover.test.mjs test/remote-backend-shell-integration.test.mjs`
  - `npm test`
- Backend suite:
  - `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
- Docs:
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/governance-behavioral-closeout-2026-03-19`
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/governance-behavioral-closeout-2026-03-19 README.md README_CN.md docs/runtime-architecture.md docs/README.md docs/archive-index.md`

## Verification Results

- `npm ci`
  - required in this verify worktree because `jiti` was missing before the first JS run.
- `node --test --test-name-pattern='.' test/governance-tools.test.mjs test/auto-recall-behavioral.test.mjs test/config-session-strategy-cutover.test.mjs test/remote-backend-shell-integration.test.mjs`
  - passed: 69 tests, 0 failures.
- `npm test`
  - passed after dependency install.
- `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
  - passed: 55 tests, 0 failures.
  - note: existing backend warning about the unused `RateLimited` enum variant remained unchanged by this scope.
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/governance-behavioral-closeout-2026-03-19`
  - result: `[OK] placeholder scan clean`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/governance-behavioral-closeout-2026-03-19 README.md README_CN.md docs/runtime-architecture.md docs/README.md docs/archive-index.md`
  - result: `[OK] stale refactor text patterns not found`
  - result: `[OK] suspicious shell-leak text not found`
- Residual naming scan:
  - non-archive docs outside this scope are clean for the removed self-improvement/reflection-module patterns after refreshing `docs/context-engine-split-2026-03-17/`;
  - remaining active-code matches are intentional alias-rejection guards in `index.ts` and their validation coverage in `test/config-session-strategy-cutover.test.mjs`.

## Changed Files Summary

- Canonical governance closeout:
  - `src/governance-tools.ts`
  - deleted `src/self-improvement-tools.ts`
  - `test/governance-tools.test.mjs`
- Behavioral-guidance canonical naming:
  - `index.ts`
  - `src/context/auto-recall-orchestrator.ts`
  - `src/context/session-exposure-state.ts`
  - `src/context/behavioral-guidance-error-signals.ts`
  - `src/context/prompt-block-renderer.ts`
  - deleted `src/context/reflection-prompt-planner.ts`
  - deleted `src/context/reflection-error-signals.ts`
  - `test/helpers/behavioral-guidance-reference.ts`
  - `test/auto-recall-behavioral.test.mjs`
- Adapter/backend internal neutral naming:
  - `src/backend-client/types.ts`
  - `src/backend-client/client.ts`
  - `src/backend-tools.ts`
  - `backend/src/lib.rs`
  - `backend/src/models.rs`
  - `backend/src/state.rs`
  - `test/remote-backend-shell-integration.test.mjs`
  - `test/config-session-strategy-cutover.test.mjs`
- Docs/archive disposition:
  - `README.md`
  - `README_CN.md`
  - `docs/runtime-architecture.md`
  - `docs/README.md`
  - `docs/archive-index.md`
  - `docs/context-engine-split-2026-03-17/context-engine-split-brainstorming.md`
  - `docs/context-engine-split-2026-03-17/context-engine-split-contracts.md`
  - `docs/context-engine-split-2026-03-17/context-engine-split-implementation-research-notes.md`
  - `docs/context-engine-split-2026-03-17/context-engine-split-2026-03-17-technical-documentation.md`
  - `docs/archive/autorecall-governance-unification-2026-03-18/`
  - current scope docs under `docs/archive/governance-behavioral-closeout-2026-03-19/`

## Final Semantics and Archive Disposition

- Final active semantics:
  - `distill` is the only trajectory-derived generation authority.
  - `autoRecall` is the only prompt-time orchestration surface.
  - `governance` is the only governance workflow surface.
- Disposition decisions:
  - remove all active governance/self-improvement alias tool registration;
  - delete all no-longer-needed wrapper modules;
  - archive the old unification scope;
  - retain the backend reflection route/storage contract and document it explicitly instead of hiding it behind more compatibility layers.
