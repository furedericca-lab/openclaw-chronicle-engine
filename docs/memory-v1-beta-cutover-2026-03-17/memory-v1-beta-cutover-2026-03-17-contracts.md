---
description: API and schema contracts for memory-v1-beta-cutover-2026-03-17.
---

# memory-v1-beta-cutover-2026-03-17 Contracts

## API Contracts

- no backend REST route changes are required for this scope;
- runtime data-plane remains:
  - `POST /v1/memories`
  - `POST /v1/recall`
  - `POST /v1/reflection/source`
  - `POST /v1/reflection/jobs`
  - `GET /v1/reflection/jobs/:id`
  - existing distill and debug routes already in scope today;
- this scope changes the plugin-side config and release contract, not the backend HTTP contract.

## Shared Types / Schemas

- package version and plugin version must both be `1.0.0-beta.0`;
- supported config surface after this cutover is:
  - `remoteBackend.*`
  - `enableManagementTools`
  - `sessionStrategy`
  - `autoCapture`
  - `autoRecall*`
  - `captureAssistant`
  - active `memoryReflection.*` fields only:
    - `injectMode`
    - `messageCount`
    - `errorReminderMaxEntries`
    - `dedupeErrorSignals`
    - `recall.*`
  - `selfImprovement.*`;
- unsupported legacy fields after this cutover:
  - `sessionMemory.enabled`
  - `sessionMemory.messageCount`
  - `memoryReflection.agentId`
  - `memoryReflection.maxInputChars`
  - `memoryReflection.timeoutMs`
  - `memoryReflection.thinkLevel`.

## Ownership and Compatibility

- this cutover is intentionally breaking and does not preserve migration parsing for the listed legacy fields;
- config parsing must fail closed on removed fields instead of mapping or silently ignoring them;
- active docs and schema text must describe the new baseline without “legacy compatibility” caveats for removed fields;
- archived docs may still mention migration behavior as historical evidence.

## Event and Streaming Contracts

- no new event stream is introduced;
- startup logging may change from warning-on-ignored-fields to hard validation failure or rejection behavior depending on parser implementation;
- runtime tool/event payloads remain unchanged.

## Error Model

- removed config fields must produce deterministic validation failures with concrete field names;
- do not fall back to warn-and-continue for removed legacy fields;
- runtime behavior must remain fail-closed when required remote principal identity is missing.

## Validation and Compatibility Rules

- `package.json` and `openclaw.plugin.json` versions must match exactly;
- `README.md`, `README_CN.md`, and `openclaw.plugin.json` must not describe removed legacy config fields as accepted input;
- tests that assert mapping or ignored-field behavior for removed fields must be deleted or rewritten to assert rejection;
- retained test/reference helper files must either:
  - move under `test/helpers/`, or
  - remain with explicit non-runtime naming and no production import path.

## Rejected Historical Shapes / Non-Goals

- no continued support for `sessionMemory.*` as an alias layer;
- no continued parsing of deprecated local reflection-generation knobs;
- no reintroduction of local session-file recovery or local memory authority;
- no backend API version bump in this scope unless implementation uncovers a real contract break.
