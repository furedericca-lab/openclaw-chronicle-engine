---
description: API and schema contracts for context-engine-split.
---

# context-engine-split Contracts

## API Contracts

This branch does **not** introduce a new public OpenClaw plugin contract. The external contract remains:
- plugin kind: `memory`
- current memory tools and self-improvement tools
- current config schema in `openclaw.plugin.json`

Internal contracts to introduce in this branch:

### Generic recall candidate provider
Request shape:
- prompt: string
- agentId: string
- sessionId: string
- scopeFilter: string[]
- limits/config snapshot:
  - topK
  - fetchLimit
  - minPromptLength
  - minRepeated
  - maxAgeDays
  - maxEntriesPerKey
  - selectionMode

Response shape:
- `candidates[]` with:
  - entry id/text/category/scope
  - score
  - source flags (`bm25`, `reranked`, etc.)
  - normalized recall key if needed for dedupe
- `selected[]` (optional if provider owns preselection) or raw candidates for orchestrator-side final selection

### Reflection recall provider
Request shape:
- prompt: string
- agentId: string
- sessionId/sessionKey: string
- includeKinds: (`invariant` | `derived`)[]
- limits/config snapshot:
  - topK
  - minPromptLength
  - minRepeated
  - minScore
  - maxAgeDays
  - maxEntriesPerKey
  - recall mode

Response shape:
- `rows[]` with representative text, score, kind, strictKey, metadata for rendering
- optional block-plan metadata for inheritance-only vs inheritance+derived modes

### Error-signal provider
Request shape:
- sessionKey: string
- maxEntries: number

Response shape:
- `signals[]` with:
  - toolName
  - summary
  - signatureHash
  - timestamp

### Context block renderer
Input:
- tag: `relevant-memories` | `inherited-rules` | `error-detected`
- rows/signals
- wrapping mode (`wrapUntrustedData`, header lines, numbering rules)

Output:
- rendered block string
- optional metadata (selected ids/count) for tests/logging

## Shared Types / Schema definitions and ownership

Ownership rules:
- Backend modules own persistence and retrieval row shapes.
- Orchestration modules own prompt-block plan/render types.
- `index.ts` owns only wiring/config-to-dependency translation.

Compatibility rule:
- No config key rename in this branch.
- `autoRecallSelectionMode: legacy` must continue to parse as `mmr`.
- `sessionMemory.*` legacy compatibility mapping must remain intact.

## Event contracts

Hook/event surfaces that must preserve behavior:
- `before_agent_start` → generic auto-recall path
- `before_prompt_build` → inherited rules + error-detected path
- `after_tool_call` → error-signal capture path
- `agent_end` → auto-capture path
- `command:new` / `command:reset` → reflection generation path

Contract rule:
- This branch may thin handler bodies, but it must not delete these active paths without replacement verification.

## Validation rules and compatibility policy

Validation rules:
- Existing tests stay green.
- New orchestration modules must be unit-testable without full plugin bootstrap where practical.
- Docs must not claim a shipped standalone ContextEngine.

Compatibility policy:
- Backward compatible by default.
- Internal module moves are allowed.
- Public tool names, config keys, and memory-slot behavior are not allowed to break in this branch.

## Security-sensitive fields and redaction/masking requirements

- Scope-filtered candidate selection must happen before rendering any prompt block.
- Error signals must remain summarized; no raw sensitive payload dumping into docs/tests.
- Reflection-derived blocks must preserve current untrusted-data framing semantics when surfaced to the model.
