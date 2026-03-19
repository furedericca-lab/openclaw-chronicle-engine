---
description: API and schema contracts for governance-behavioral-closeout-2026-03-19.
---

# governance-behavioral-closeout-2026-03-19 Contracts

## API Contracts

- Governance tool registration is canonical-only:
  - `governance_log`
  - `governance_review`
  - `governance_extract_skill`
- `memory_recall_debug` keeps the same backend debug routes, but the active tool surface now uses:
  - `channel: "generic" | "behavioral"`
  - `behavioralMode` for `channel="behavioral"`
- Public READMEs and runtime docs must describe governance-only tooling and behavioral-guidance semantics; no active doc may advertise `self_improvement_*` tool ids.

## Shared Types / Schemas

- Removed compatibility shims:
  - `src/self-improvement-tools.ts`
  - `src/context/reflection-prompt-planner.ts`
  - `src/context/reflection-error-signals.ts`
- Canonical adapter/runtime internal naming is behavioral-guidance oriented:
  - `BehavioralRecallMode`
  - `recallBehavioral*`
  - `behavioralCount`
  - `BehavioralGuidanceErrorSignal`
- Backend compatibility boundary intentionally remains:
  - HTTP routes `/v1/recall/reflection` and `/v1/debug/recall/reflection`
  - persisted category `reflection`
  - persisted/storage field `reflection_kind`
  - backend response field `reflectionCount`

## Event and Streaming Contracts

- `before_agent_start`: generic autoRecall planning only.
- `before_prompt_build`: behavioral-guidance recall plus error reminder blocks only.
- `after_tool_call`: behavioral-guidance error-signal capture only.
- `/new` and `/reset` note injection keeps carried-forward focus text, not “reflection-derived focus” wording.

## Error Model

- `parsePluginConfig(...)` must reject the removed compatibility aliases rather than normalizing them:
  - `sessionMemory`
  - `memoryReflection`
  - `selfImprovement`
  - `autoRecallExcludeReflection`
  - `autoRecallBehavioral.injectMode=inheritance-only|inheritance+derived`
  - `autoRecallBehavioral.recall.includeKinds[]=invariant|derived`
- Plugin-side manual writes to `category=reflection` must fail with a behavioral-guidance-facing error code (`behavioral_category_reserved`) before the backend call.
- Backend write routes still reject manual `category=reflection` mutations at the data-plane boundary.

## Validation and Compatibility Rules

- `openclaw.plugin.json` remains the canonical config schema and was already aligned; this scope does not reintroduce hidden alias compatibility behind the schema.
- Governance backlog initialization is `.governance/` only; `.learnings/` read-through compatibility is removed in this closeout.
- The previous large unification scope is archived under `docs/archive/autorecall-governance-unification-2026-03-18/`.
- Verification commands for this scope:
  - `npm test`
  - `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/governance-behavioral-closeout-2026-03-19`
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/governance-behavioral-closeout-2026-03-19 README.md README_CN.md docs/runtime-architecture.md docs/README.md docs/archive-index.md`
