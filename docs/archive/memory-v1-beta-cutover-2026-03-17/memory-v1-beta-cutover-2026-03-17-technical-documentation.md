---
description: Canonical technical architecture for memory-v1-beta-cutover-2026-03-17.
---

# memory-v1-beta-cutover-2026-03-17 Technical Documentation

## Canonical Architecture

- backend remains the only runtime authority for memory storage, retrieval, transcript-backed reflection source loading, and async memory/distill jobs;
- plugin runtime remains responsible for transport, prompt-local orchestration, and caller-scoped tool exposure;
- this scope narrows the operator and repository contract so it matches that architecture without migration scaffolding.

## Key Constraints and Non-Goals

- do not change backend route semantics;
- do not preserve parse-time compatibility for removed legacy fields;
- do not reintroduce runtime ambiguity between backend authority and local test/reference helpers;
- do not leave user-visible placeholder text in shipped tool output.

## Module Boundaries and Data Flow

- `index.ts` owns config parsing and runtime registration;
- `openclaw.plugin.json` owns public schema/help text;
- `README.md` and `README_CN.md` own operator-facing documentation;
- `package.json` and `openclaw.plugin.json` own release version identity;
- `test/config-session-strategy-migration.test.mjs`, `test/memory-reflection.test.mjs`, `test/query-expander.test.mjs`, and `test/self-improvement.test.mjs` own the relevant regression gates;
- relocated or renamed helper modules must remain test-only and must not be imported by supported runtime paths.

## Interfaces and Contracts

- accepted config after this cutover is the exact active schema described in `openclaw.plugin.json`;
- removed fields must fail validation or be rejected rather than parsed into compatibility state;
- active release metadata must consistently state `1.0.0-beta.0`;
- helper relocation must preserve test behavior without changing supported runtime imports.

## Ownership Boundary

- runtime authority:
  - backend data-plane and job lifecycle;
- prompt-local authority:
  - heuristics, selection, and formatting over backend-returned rows;
- test/reference only:
  - lexical expansion references and reflection-store references if they are not imported by runtime code.

## Security and Reliability

- fail closed on removed config fields to avoid silent misconfiguration;
- keep caller-scoped tool boundaries and principal checks unchanged;
- avoid doc/schema drift that could cause unsafe operator assumptions about accepted config.

## Observability and Error Handling

- startup validation errors should cite exact removed field names;
- removal of compatibility warnings should coincide with stronger validation behavior;
- version/reporting surfaces should no longer mix old and new project-line numbers.

## Test Strategy

- parser/config tests for removed-field rejection and active-field success;
- Node tests for relocated helper imports and self-improvement output text;
- focused `rg` scans across active files for removed config fields and stale version strings;
- full `npm test` before closeout.
