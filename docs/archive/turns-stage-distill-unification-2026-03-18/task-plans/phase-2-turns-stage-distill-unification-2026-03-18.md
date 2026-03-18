---
description: Phase 2 plan for turns-stage-distill-unification-2026-03-18.
---

# Phase 2 — TS/plugin cleanup

## Goal
Delete command-triggered reflection generation and leave cadence-driven distill as the only automatic generation path in TS/runtime.

## Tasks
- Remove `runMemoryReflection` command-generation code from `index.ts`.
- Remove `command:new` / `command:reset` reflection hook registration.
- Remove now-dead trigger dedupe/source-loading/enqueue code.
- Remove unused client/tool surfaces tied only to reflection jobs.
- Remove user-visible wording or config/test traces that imply rollback compatibility for deleted reflection-generation behavior.
- Rewrite/delete affected TS tests.

## Target files
- `index.ts`
- `src/backend-tools.ts`
- `src/backend-client/client.ts`
- `src/backend-client/types.ts`
- `test/remote-backend-shell-integration.test.mjs`
- `test/memory-reflection.test.mjs`

## Verification
- `npm test -- --test-name-pattern="reflection|distill|session-strategy"`
- `rg -n "reflection/source|reflection/jobs|command:new|command:reset" index.ts src test`

## Done definition
- no runtime registration remains for command-triggered reflection generation
- no TS test still expects `/new` / `/reset` reflection enqueue
- distill cadence tests still pass
