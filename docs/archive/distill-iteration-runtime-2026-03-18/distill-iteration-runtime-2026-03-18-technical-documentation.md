---
description: Canonical technical architecture for distill-iteration-runtime-2026-03-18.
---

# distill-iteration-runtime-2026-03-18 Technical Documentation

## Canonical Architecture

1. runtime receives `agent_end`;
2. runtime resolves actor identity and appends transcript rows to backend;
3. runtime counts completed user turns for the session;
4. when the configured cadence boundary is crossed, runtime enqueues one backend-native `session-transcript` distill job;
5. backend loads transcript rows, synthesizes deterministic span/window candidates, reduces them, and persists artifacts and optional memory rows.

## Key Constraints and Non-Goals

- distill output is English-only in this scope;
- runtime cadence must not create a new transcript authority path;
- no model-backed extraction is introduced.

## Module Boundaries and Data Flow

- runtime/config: [index.ts](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/index.ts)
- plugin schema: [openclaw.plugin.json](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/openclaw.plugin.json)
- backend execution: [backend/src/state.rs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/src/state.rs)
- backend contracts: [backend/tests/phase2_contract_semantics.rs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/tests/phase2_contract_semantics.rs)
- shell integration: [test/remote-backend-shell-integration.test.mjs](/root/.openclaw/workspace/plugins/openclaw-chronicle-engine/test/remote-backend-shell-integration.test.mjs)

## Interfaces and Contracts

- The backend job DTO shape does not change.
- Runtime only adds an optional config surface that influences enqueue timing and backend options.

## Ownership Boundary

- Runtime: cadence state and enqueue timing
- Backend: source resolution, reduction, persistence

## Security and Reliability

- automatic distill reuses existing principal-authenticated backend calls
- automatic enqueue is fail-open and idempotent

## Observability and Error Handling

- transcript append and automatic distill enqueue log separately
- backend job failures remain visible through existing status routes

## Test Strategy

- config parse test for `distill`
- runtime shell integration for automatic cadence enqueue
- backend contract test for multi-message evidence aggregation and structured summary
