---
description: Scope boundaries and milestones for distill-iteration-runtime-2026-03-18.
---

# distill-iteration-runtime-2026-03-18 Scope and Milestones

## In Scope

- deterministic backend distill quality upgrades:
  - multi-message evidence aggregation
  - span/window-level candidate synthesis
  - stronger tags/category/importance heuristics
  - rule-based English summary compression
  - cross-message dedupe/merge
- optional runtime automatic distill cadence based on completed user turns
- config schema, tests, and docs for the new cadence/runtime behavior

## Out of Scope

- language-adaptive extraction or language-specific prompt selection
- backend model/provider-backed map phase
- sidecar workers, queue files, or local transcript authority
- changing the external distill status payload shape

## Milestones

1. Freeze contracts and scope boundaries for deterministic distill upgrades plus cadence-based runtime enqueue.
2. Implement backend reducer upgrades and prove span/evidence/summary behavior in backend tests.
3. Implement runtime `everyTurns` automatic distill enqueue and prove shell integration/config behavior.
4. Refresh docs and run scope verification scans.

## Dependencies

- existing `agent_end -> session-transcripts/append` runtime path in [index.ts](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/index.ts)
- existing backend-native distill job family in [backend/src/state.rs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/src/state.rs)
- existing shell integration harness in [test/remote-backend-shell-integration.test.mjs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/test/remote-backend-shell-integration.test.mjs)

## Exit Criteria

- backend distill no longer behaves as message-only truncation on the tested incident-style paths
- runtime can automatically enqueue one distill job every configured N user turns
- management/manual distill surfaces remain compatible
- `npm test` passes
- targeted `cargo test distill_ -- --nocapture` passes
- scope doc scans pass

## Archive / Handoff Note

- archive after code, tests, docs, and scans are complete
