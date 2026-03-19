---
description: Canonical technical architecture for autorecall-governance-unification-2026-03-18.
---

# autorecall-governance-unification-2026-03-18 Technical Documentation

## Canonical Architecture
- Runtime authority split after this scope:
  - `distill`
    - only trajectory-derived generation authority
    - backend-owned enqueue, execution, persistence, and artifact reduction
  - `autoRecall`
    - only prompt-time recall/injection orchestration surface
    - contains two profiles:
      - context recall
      - behavioral guidance
  - `governance`
    - backlog/review/extraction/promotion workflow tools and files
- Backend implementation detail:
  - behavioral guidance still reads the backend reflection-recall route and backend-managed reflection rows until a later backend contract scope changes that storage/API.

## Key Constraints and Non-Goals
- Do not reintroduce any local trajectory-derived generation path under reflection or governance naming.
- Do not let governance own prompt-time reminder injection; that belongs to behavioral autoRecall guidance.
- Do not change backend route or storage contracts unless strictly needed for adapter compatibility.

## Module Boundaries and Data Flow
- Plugin runtime:
  - `index.ts`
    - parses the canonical config surface and rejects removed legacy config aliases
    - wires remote backend client
    - registers autoRecall hooks, governance tools, and distill cadence
- Prompt-time context modules:
  - `src/context/auto-recall-orchestrator.ts`
    - owns both generic context recall and behavioral-guidance planning
  - `src/context/session-exposure-state.ts`
    - owns per-session suppression state for autoRecall context rows, behavioral rows, and recent tool-error reminder signals
  - `src/context/prompt-block-renderer.ts`
    - renders `<relevant-memories>`, `<behavioral-guidance>`, and `<error-detected>`
- Governance workflow modules:
  - `src/governance-tools.ts`
    - owns governance backlog file initialization
    - governance log/review/extract tool registration
    - transitional alias registration if kept
- Backend adapter modules:
  - `src/backend-client/client.ts`
  - `src/backend-client/types.ts`
  - `src/backend-tools.ts`
    - may still expose backend reflection details internally, but current docs and user-facing strings should favor behavioral autoRecall wording where practical

## Interfaces and Contracts
- Canonical config surface:
  - `sessionStrategy: "autoRecall" | "systemSessionMemory" | "none"`
  - `autoRecall` generic config
  - `autoRecallBehavioral` config
  - `governance` config
- Removed legacy config surface:
  - `memoryReflection`
  - `selfImprovement`
  - `sessionStrategy: "memoryReflection"`
- Canonical prompt tags:
  - `<relevant-memories>`
  - `<behavioral-guidance>`
  - `<error-detected>`
- Canonical governance tools:
  - `governance_log`
  - `governance_review`
  - `governance_extract_skill`

## Operational Behavior
- Startup:
  - create remote backend client
  - register governance tools
  - register autoRecall hooks
  - optionally register behavioral reminder/bootstrap hooks
- Turn flow:
  - `before_agent_start` may inject context recall
  - `after_tool_call` records new tool-error signals
  - `before_prompt_build` may inject behavioral guidance + recent tool-error reminders
  - `agent_end` appends transcript, optionally enqueues distill cadence, and optionally auto-captures memories
- Session cleanup:
  - `session_end` and `before_reset` clear autoRecall suppression and recent guidance error state

## Security and Reliability
- Prompt-time remote recall remains fail-open on missing principal identity or backend recall failure.
- Governance skill extraction keeps output paths sanitized under the workspace.
- Governance backlog file creation remains local-workspace-only.
- Distill remains isolated from governance and autoRecall rename work.

## Test Strategy
- JS:
  - config parser tests for canonical names and rejection of removed legacy config aliases
  - prompt-time autoRecall tests for context + behavioral guidance channels
  - governance tool tests for logging/review/extract and file migration behavior
  - integration tests for runtime hook wiring and fail-open behavior
- Rust:
  - re-run `backend/tests/phase2_contract_semantics.rs` only when backend-facing code changes
- Docs:
  - placeholder scan
  - residual scan over scope docs and updated public docs

## Changed Files Summary
- Runtime/config:
  - `index.ts`
  - `openclaw.plugin.json`
  - `package.json`
- Prompt-time orchestration:
  - `src/context/auto-recall-orchestrator.ts`
  - `src/context/behavioral-guidance-error-signals.ts`
  - `src/context/reflection-error-signals.ts`
  - `src/context/reflection-prompt-planner.ts`
  - `src/context/prompt-block-renderer.ts`
  - `src/context/session-exposure-state.ts`
- Governance workflow:
  - `src/governance-tools.ts`
  - `src/self-improvement-tools.ts`
- Public docs and architecture references:
  - `README.md`
  - `README_CN.md`
  - `docs/runtime-architecture.md`
  - `docs/context-engine-split-2026-03-17/*`
  - scope docs under `docs/autorecall-governance-unification-2026-03-18/*`
- Tests/helpers:
  - `test/auto-recall-behavioral.test.mjs`
  - `test/governance-tools.test.mjs`
  - `test/config-session-strategy-cutover.test.mjs`
  - `test/remote-backend-shell-integration.test.mjs`
  - `test/helpers/openclaw-extension-api-stub.mjs`
  - `test/helpers/reflection-reference.ts`

## Reviewer Blocker Follow-up Changed Files
- Runtime/config:
  - `index.ts`
- Public docs:
  - `README.md`
  - `README_CN.md`
  - `docs/context-engine-split-2026-03-17/context-engine-split-2026-03-17-technical-documentation.md`
- Scope docs:
  - `docs/autorecall-governance-unification-2026-03-18/autorecall-governance-unification-2026-03-18-contracts.md`
  - `docs/autorecall-governance-unification-2026-03-18/autorecall-governance-unification-2026-03-18-technical-documentation.md`
  - `docs/autorecall-governance-unification-2026-03-18/autorecall-governance-unification-2026-03-18-implementation-research-notes.md`
  - `docs/autorecall-governance-unification-2026-03-18/autorecall-governance-unification-2026-03-18-scope-milestones.md`
  - `docs/autorecall-governance-unification-2026-03-18/task-plans/4phases-checklist.md`
  - `docs/autorecall-governance-unification-2026-03-18/task-plans/phase-2-autorecall-governance-unification-2026-03-18.md`
- Tests:
  - `test/config-session-strategy-cutover.test.mjs`
  - `test/remote-backend-shell-integration.test.mjs`

## Verification Results
- JavaScript:
  - `node --test --test-name-pattern='.' test/config-session-strategy-cutover.test.mjs test/auto-recall-behavioral.test.mjs test/governance-tools.test.mjs test/remote-backend-shell-integration.test.mjs`
  - Passed: 65 tests, 0 failures.
- Rust:
  - `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
  - Passed: 55 tests, 0 failures.
  - Non-blocking note: compile emitted the existing backend warning `RateLimited is never constructed`.
- Docs:
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/autorecall-governance-unification-2026-03-18`
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/autorecall-governance-unification-2026-03-18 README.md`
  - Passed: both scans clean.
- Reviewer blocker follow-up (legacy config alias removal):
  - JavaScript:
    - `node --test test/config-session-strategy-cutover.test.mjs test/auto-recall-behavioral.test.mjs test/remote-backend-shell-integration.test.mjs`
    - Passed: 62 tests, 0 failures.
  - Docs:
    - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/autorecall-governance-unification-2026-03-18`
    - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/autorecall-governance-unification-2026-03-18 README.md README_CN.md docs/context-engine-split-2026-03-17/context-engine-split-2026-03-17-technical-documentation.md`
    - `rg -n 'legacy alias is still accepted|parse compatibility aliases|compatibility alias for autoRecall|maps to "autoRecall"|maps to autoRecallBehavioral' README.md README_CN.md docs/context-engine-split-2026-03-17/context-engine-split-2026-03-17-technical-documentation.md index.ts test`
    - Result: placeholder scan clean, post-refactor scan clean, residual grep returned no matches.
  - Backend:
    - Not rerun in the blocker follow-up because no backend files changed.

## Compatibility and Remaining Gaps
- Intentional transitional compatibility shims remain:
  - tools: `self_improvement_log`, `self_improvement_review`, `self_improvement_extract_skill`
  - module shims: `src/self-improvement-tools.ts`, `src/context/reflection-prompt-planner.ts`, `src/context/reflection-error-signals.ts`
- Removed legacy config aliases no longer parse:
  - `memoryReflection`
  - `selfImprovement`
  - `sessionStrategy: "memoryReflection"`
- Intentional backend internal details remain unchanged in this scope:
  - `/v1/recall/reflection`
  - `/v1/debug/recall/reflection`
  - backend category/trace kind `reflection`
- The only notable residual issue from the initial full-scope verification is the pre-existing Rust dead-code warning for `RateLimited`; no failing behavior was observed, and the follow-up patch did not touch backend code.
