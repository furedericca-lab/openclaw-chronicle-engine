---
description: Implementation research notes for autorecall-governance-unification-2026-03-18.
---

# autorecall-governance-unification-2026-03-18 Implementation Research Notes

## Discovery Baseline (Pre-change)
- Prompt-time orchestration is split:
  - `src/context/auto-recall-orchestrator.ts` owns generic `<relevant-memories>` injection.
  - `src/context/reflection-prompt-planner.ts` owns former reflection inherited-rule and error-reminder injection.
  - `src/context/session-exposure-state.ts` tracks both `autoRecallState` and `reflectionRecallState`.
- Runtime wiring in `index.ts` still exposes separate concepts:
  - `sessionStrategy: "memoryReflection" | "systemSessionMemory" | "none"`
  - `memoryReflection` config
  - `selfImprovement` config
  - dedicated self-improvement bootstrap/reminder hook registration
  - dedicated reflection prompt planner hook registration
- Governance workflow tools are still named under self-improvement:
  - `src/self-improvement-tools.ts`
  - tool ids `self_improvement_log`, `self_improvement_review`, `self_improvement_extract_skill`
  - canonical storage currently points at `.learnings/LEARNINGS.md` and `.learnings/ERRORS.md`
- Public docs/tests still present these concepts as current architecture:
  - `README.md`
  - `README_CN.md`
  - `docs/runtime-architecture.md`
  - `docs/context-engine-split-2026-03-17/*`
  - `test/memory-reflection.test.mjs`
  - `test/self-improvement.test.mjs`
  - `test/remote-backend-shell-integration.test.mjs`
  - `openclaw.plugin.json`

## Gap Analysis
1. The current runtime violates the target architecture.
   Evidence: `index.ts` constructs both `createAutoRecallPlanner` and `createReflectionPromptPlanner`, and only the latter participates in `before_prompt_build`.

2. Self-improvement still incorrectly owns reminder/bootstrap behavior that is really agent behavioral guidance.
   Evidence: `index.ts` injects `SELF_IMPROVEMENT_REMINDER.md` on `agent:bootstrap` and `/note self-improvement (before reset)` on command hooks.

3. Governance workflow tooling is semantically right but named and stored under the wrong owner.
   Evidence: `src/self-improvement-tools.ts` descriptions already reference governance backlog, but the module name, tool ids, errors, and file paths still say self-improvement / `.learnings`.

4. Reflection terminology is still active on current prompt-time public surfaces rather than being demoted to backend implementation detail.
   Evidence: `openclaw.plugin.json` documents `memoryReflection.*`, `<inherited-rules>`, and reflection-specific help text; tests assert the same terms.

## Candidate Designs and Trade-offs
### Option A: Thin rename wrappers only
- Lowest code churn.
- Leaves the runtime split intact and fails the target architecture.

### Option B: AutoRecall behavioral-guidance unification plus governance migration
- Moderate churn limited to plugin/runtime/docs/tests.
- Keeps backend wire stable while changing the public architecture to the requested model.
- Best fit for this scope.

### Option C: Full backend rename
- Highest churn and strongest semantic purity.
- Pulls Rust/API/persistence migration into a scope that does not require it.

## Selected Design
- Unify prompt-time orchestration around autoRecall:
  - generic context recall stays in autoRecall.
  - former reflection recall/injection becomes behavioral autoRecall guidance.
  - recent tool-error reminder injection remains prompt-local and joins that behavioral channel.
- Split former self-improvement responsibilities:
  - governance owns backlog/review/extract/promotion tools and files.
  - behavioral autoRecall owns reminder/bootstrap/note injection because those are agent-guidance behaviors.
- Keep backend reflection semantics behind adapters for now:
  - adapter names may still call backend `recallReflection`.
  - public docs/config/tests stop presenting reflection as a top-level current architecture term.

## Validation Plan
- JavaScript tests:
  - `node --test --test-name-pattern='.' test/config-session-strategy-cutover.test.mjs test/auto-recall-behavioral.test.mjs test/governance-tools.test.mjs test/remote-backend-shell-integration.test.mjs`
- Rust tests:
  - `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
- Docs hygiene:
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/autorecall-governance-unification-2026-03-18`
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/autorecall-governance-unification-2026-03-18 README.md`
- Residual terminology scans:
  - targeted `rg -n "self_improvement|memoryReflection|inherited-rules|reflection-recall" README.md README_CN.md docs/runtime-architecture.md docs/context-engine-split-2026-03-17 index.ts src test openclaw.plugin.json`

## Risks and Assumptions
- Assumption: keeping backend reflection routes and category names is acceptable as long as public plugin architecture no longer presents them as peers.
- Risk: test renames may require package script updates and cross-file import path cleanup.
- Risk: governance file-path migration can create dual-write ambiguity; closeout notes must record the chosen compatibility rule.

## Implemented State (2026-03-18)
- Prompt-time orchestration is now canonically centered on autoRecall:
  - `src/context/auto-recall-orchestrator.ts` owns both generic context recall and behavioral-guidance planning.
  - `src/context/prompt-block-renderer.ts` renders `<relevant-memories>`, `<behavioral-guidance>`, and `<error-detected>`.
  - `src/context/session-exposure-state.ts` exposes canonical behavioral-guidance state while preserving legacy reflection aliases for compatibility.
  - `src/context/behavioral-guidance-error-signals.ts` is the canonical error-signal helper, with `src/context/reflection-error-signals.ts` left as a compatibility shim.
- Runtime/config surfaces are canonicalized:
  - `index.ts` now treats `sessionStrategy: "autoRecall"` and `autoRecallBehavioral` as the canonical behavioral surface.
  - legacy `sessionStrategy: "memoryReflection"`, `memoryReflection`, and `selfImprovement` are rejected instead of being parsed as transitional aliases.
  - `openclaw.plugin.json` documents `autoRecall`, `autoRecallBehavioral`, `autoRecallExcludeBehavioral`, and `governance` as the active schema.
- Governance now owns backlog workflow tools/files:
  - `src/governance-tools.ts` registers `governance_log`, `governance_review`, and `governance_extract_skill`.
  - `src/self-improvement-tools.ts` remains only as a compatibility re-export surface.
  - governance backlog files are canonically initialized under `.governance/`, with legacy `.learnings/` treated as compatibility input only.
- Public docs/tests were updated to match the new architecture:
  - `README.md`, `README_CN.md`, `docs/runtime-architecture.md`, and `docs/context-engine-split-2026-03-17/*` present autoRecall plus governance as the active model.
  - test filenames were renamed to `test/auto-recall-behavioral.test.mjs` and `test/governance-tools.test.mjs`.
  - `package.json` test script now uses the renamed test files.

## Verification Results
- JavaScript:
  - `node --test --test-name-pattern='.' test/config-session-strategy-cutover.test.mjs test/auto-recall-behavioral.test.mjs test/governance-tools.test.mjs test/remote-backend-shell-integration.test.mjs`
  - Result: passed, 65 tests / 0 failures.
- Rust:
  - `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
  - Result: passed, 55 tests / 0 failures.
  - Note: existing backend warning `RateLimited is never constructed` was emitted during compile; no new scope regression was indicated.
- Docs:
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/autorecall-governance-unification-2026-03-18`
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/autorecall-governance-unification-2026-03-18 README.md`
  - Result: both scans passed.
- Reviewer blocker follow-up verification:
  - `node --test test/config-session-strategy-cutover.test.mjs test/auto-recall-behavioral.test.mjs test/remote-backend-shell-integration.test.mjs`
  - Result: passed, 62 tests / 0 failures.
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/autorecall-governance-unification-2026-03-18`
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/autorecall-governance-unification-2026-03-18 README.md README_CN.md docs/context-engine-split-2026-03-17/context-engine-split-2026-03-17-technical-documentation.md`
  - `rg -n 'legacy alias is still accepted|parse compatibility aliases|compatibility alias for autoRecall|maps to "autoRecall"|maps to autoRecallBehavioral' README.md README_CN.md docs/context-engine-split-2026-03-17/context-engine-split-2026-03-17-technical-documentation.md index.ts test`
  - Result: scans passed and residual grep returned no matches.
  - Backend tests were intentionally not rerun in this follow-up because no backend files changed.

## Remaining Compatibility Notes
- Backend routes/storage still use reflection-labeled contracts internally:
  - `/v1/recall/reflection`
  - `/v1/debug/recall/reflection`
  - backend category/trace kind `reflection`
- Removed legacy config aliases remain intentionally unsupported:
  - `sessionStrategy: "memoryReflection"`
  - `memoryReflection`
  - `selfImprovement`
- Transitional tool/module aliases remain intentionally supported:
  - `self_improvement_log`
  - `self_improvement_review`
  - `self_improvement_extract_skill`
  - `src/self-improvement-tools.ts`
  - `src/context/reflection-prompt-planner.ts`
  - `src/context/reflection-error-signals.ts`

## Reviewer Blocker Resolution (2026-03-18)
- Problem:
  - The prior scope docs/runtime claimed legacy config alias compatibility for `sessionStrategy: "memoryReflection"`, `memoryReflection`, and `selfImprovement`, but `openclaw.plugin.json` already rejected those surfaces before runtime alias mapping could apply.
- Resolution:
  - Removed runtime parsing/mapping for those legacy config surfaces from `index.ts`.
  - Converted tests from alias-success expectations to explicit rejection checks.
  - Updated active docs and scope docs so only canonical config surfaces are documented as supported.
- Non-goal preserved:
  - Legacy tool-name aliases (`self_improvement_*`) remain intentionally supported and documented as compatibility shims only.
