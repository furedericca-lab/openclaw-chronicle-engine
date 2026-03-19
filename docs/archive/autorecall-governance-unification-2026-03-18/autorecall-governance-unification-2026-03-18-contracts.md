---
description: API and schema contracts for autorecall-governance-unification-2026-03-18.
---

# autorecall-governance-unification-2026-03-18 Contracts

## API Contracts
- Plugin public prompt-time contract:
  - `autoRecall` remains the generic context recall switch for `<relevant-memories>`.
  - Behavioral recall/injection is presented as an autoRecall behavioral-guidance profile, not as a peer `reflection` architecture concept.
  - Backend `/v1/recall/reflection` and `/v1/debug/recall/reflection` stay adapter-level implementation details for this scope.
- Tool contract changes:
  - New canonical governance tools replace `self_improvement_*` naming:
    - `governance_log`
    - `governance_review`
    - `governance_extract_skill`
  - Legacy `self_improvement_log`, `self_improvement_review`, and `self_improvement_extract_skill` may remain as documented transitional aliases that call the same implementation.
- Config contract changes:
  - New canonical public config names:
    - `sessionStrategy: "autoRecall" | "systemSessionMemory" | "none"`
    - `autoRecallBehavioral`
    - `governance`
  - Removed legacy config surfaces:
    - `sessionStrategy: "memoryReflection"` is rejected.
    - `memoryReflection` is rejected.
    - `selfImprovement` is rejected.

## Shared Types / Schemas
- Canonical prompt-time profiles:
  - context autoRecall profile
    - source: `recallGeneric`
    - output tag: `<relevant-memories>`
  - behavioral autoRecall profile
    - source: adapter-mapped backend reflection recall
    - output tag: `<behavioral-guidance>`
    - optional `<error-detected>` appendix sourced from recent tool-error signals
- Canonical governance storage:
  - governance backlog directory is plugin-owned and local to the workspace.
  - legacy `.learnings/` remains read-compatible only during migration if present.
- Shared compatibility rules:
  - backend memory category `reflection` is not renamed in this scope.
  - backend retrieval trace kind `reflection` is not renamed in this scope.
  - public docs/tests/config should treat those names as internal backend details.

## Event and Streaming Contracts
- `before_agent_start`
  - unchanged usage for generic autoRecall context recall.
- `after_tool_call`
  - captures recent tool-error signals for the behavioral autoRecall channel.
- `before_prompt_build`
  - behavioral autoRecall assembles `<behavioral-guidance>` and `<error-detected>` blocks.
- `session_end` and `before_reset`
  - clear both generic and behavioral autoRecall session suppression state.
- `agent:bootstrap`, `command:new`, `command:reset`
  - reminder/bootstrap behavior is owned by behavioral autoRecall guidance, not governance workflow tooling.

## Error Model
- Missing runtime principal identity continues to skip prompt-time remote recall rather than fail the turn.
- Backend recall failures for the behavioral channel continue to fail open and log warnings.
- Governance tool failures continue returning structured tool `details.error` codes; canonical codes now use `governance_*`, while legacy tool ids call the same implementations.

## Validation and Compatibility Rules
- Do not expose reflection as a peer prompt-time architecture term in active README/runtime docs/config schema/tests after this change.
- Do not expose self-improvement as a peer workflow/tooling term in active README/runtime docs/config schema/tests after this change.
- Preserve backend-visible reflection storage and endpoint names unless explicitly wrapped as internal compatibility details.
- Remaining compatibility aliases are limited to tool/module shims, not config parsing.
