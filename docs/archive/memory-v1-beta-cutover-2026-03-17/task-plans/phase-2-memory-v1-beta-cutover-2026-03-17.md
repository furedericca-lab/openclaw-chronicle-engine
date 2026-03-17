---
description: Task list for memory-v1-beta-cutover-2026-03-17 phase 2.
---

# Tasks: memory-v1-beta-cutover-2026-03-17 Phase 2

## Input
- Canonical sources:
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/index.ts
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/openclaw.plugin.json
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/test/config-session-strategy-migration.test.mjs
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/test/memory-reflection.test.mjs
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/test/remote-backend-shell-integration.test.mjs

## Canonical architecture / Key constraints
- parser and schema must reject removed fields instead of warning and continuing;
- active runtime behavior for supported fields must remain unchanged;
- release remains backend-owned and remote-only.

## Phase 2: Legacy Config Contract Removal
Goal: remove migration-only config parsing and schema support.

Definition of Done: removed fields are no longer accepted by parser/schema/docs/tests, and current runtime behavior for supported fields still passes.

Tasks:
- [x] T021 [Config] Remove `sessionMemory.*` compatibility parsing and deprecated `memoryReflection.agentId/maxInputChars/timeoutMs/thinkLevel` support from `index.ts`.
  - DoD: parsing code, warning code, and any compatibility-carried state are removed or replaced with validation failure behavior.
- [x] T022 [P] [Config] Update `openclaw.plugin.json` and active READMEs to remove removed-field help text and migration caveats.
  - DoD: schema/help/docs describe only the supported post-cutover config surface.
- [x] T023 [P] [QA] Rewrite config tests to assert the new contract.
  - DoD: mapping/ignored-field tests are deleted or rewritten; `npm test` passes or failures are documented with unblock plan.

Checkpoint: the active config contract matches the new-project baseline without migration aliases.

## Evidence

- `index.ts` now rejects removed config fields fail-closed.
- `openclaw.plugin.json`, `README.md`, and `README_CN.md` no longer advertise removed migration aliases.
- Parser/config regression tests were rewritten to assert rejection rather than compatibility mapping.

## Verification Commands

- `npm test`
- `rg -n "sessionMemory.enabled|sessionMemory.messageCount|memoryReflection\\.agentId|memoryReflection\\.maxInputChars|memoryReflection\\.timeoutMs|memoryReflection\\.thinkLevel" package.json package-lock.json openclaw.plugin.json README.md README_CN.md index.ts test src`

## Dependencies & Execution Order
- Depends on Phase 1.
- T021 should land before or together with T022/T023.
