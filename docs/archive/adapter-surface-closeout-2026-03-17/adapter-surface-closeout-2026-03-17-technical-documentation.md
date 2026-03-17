---
description: Canonical technical architecture for adapter-surface-closeout-2026-03-17.
---

# adapter-surface-closeout-2026-03-17 Technical Documentation

## Canonical Architecture

This scope preserves the current runtime split and narrows the remaining shell-surface ambiguity:

1. `backend/src/*` remains the only authority layer.
   - owns persistence, recall, ranking, scope derivation, ACL, reflection execution, transcript persistence, and distill execution.
   - already exposes ordinary recall routes, debug recall routes, reflection jobs, transcript append, and distill jobs.

2. `index.ts`, `src/backend-client/*`, and `src/backend-tools.ts` remain the adapter/shell layer.
   - own hook registration, tool registration, runtime identity handoff, auth header wiring, retry, and route-level fail-open/fail-closed behavior.
   - after this scope, the adapter layer is also responsible for the selected management-gated surfaces for distill and recall debug trace.

3. `src/context/*` remains the prompt-time orchestration layer.
   - owns prompt gating, block rendering, session-local exposure suppression, and error reminder state.
   - does not own backend ranking or backend debugging semantics.

4. residual local TS helpers must be explicitly classified.
   - production prompt-local seam;
   - compatibility shim;
   - test/reference-only helper;
   - removable unused residue.

## Key constraints and non-goals

- no local-authority runtime or local scope/ACL authority may return;
- ordinary `/v1/recall/*` DTOs remain narrow and stable;
- debug trace stays on explicit debug surfaces only;
- management/debug tool exposure should be gated under `enableManagementTools` unless a narrower explicit rule is documented;
- config cleanup must preserve a clear compatibility story;
- this scope does not redesign backend contracts beyond the adapter-layer methods needed to consume existing Rust routes.

## Phase 1 frozen target file map

| Path | Frozen disposition | Planned phase |
|---|---|---|
| `src/backend-tools.ts` | Add only management-gated `memory_distill_enqueue`, `memory_distill_status`, and `memory_recall_debug`; keep ordinary tool behavior unchanged when management tools are disabled. | 2 |
| `src/backend-client/types.ts` | Model backend-owned distill/debug DTOs without widening ordinary recall DTOs. | 2 |
| `src/backend-client/client.ts` | Add typed debug recall methods and keep distill/status transport aligned with existing Rust routes and error normalization. | 2 |
| `index.ts` | Keep `remoteBackend.enabled=true` mandatory; remove or demote dead local reflection-generation helpers without restoring local fallback runtime behavior. | 3 |
| `openclaw.plugin.json` | Mark stale `memoryReflection.agentId`, `maxInputChars`, `timeoutMs`, and `thinkLevel` as deprecated/ignored compatibility fields; keep supported reflection orchestration fields accurate. | 3-4 |
| `package.json` | Verify and clean residual local-RAG keywords/dependencies only after import/use-site proof shows they are non-runtime. | 4 |
| `README.md` | Align shipped behavior and operator guidance with the implemented shell surface and deprecation story. | 4 |
| `README_CN.md` | Mirror the README alignment for the Chinese runtime/operator narrative. | 4 |
| `test/remote-backend-shell-integration.test.mjs` | Add shell-surface coverage for distill enqueue/status and recall debug trace, including success and fail-closed management cases. | 2 |
| `test/backend-client-retry-idempotency.test.mjs` | Preserve backend-client retry and idempotency assumptions while adapter debug methods are added. | 2 |
| `test/memory-reflection.test.mjs` | Prove reflection compatibility and dead-local-helper cleanup does not break supported enqueue-only behavior. | 3 |
| `test/config-session-strategy-migration.test.mjs` | Prove config compatibility for deprecated `memoryReflection` fields and supported session strategy behavior. | 3 |
| `backend/tests/phase2_contract_semantics.rs` | Remain the backend-facing contract guard whenever adapter DTO assumptions are touched. | 2-4 |

### Backend-native distill surfaced through the shell

Selected shell behavior:

- backend remains the only owner of:
  - transcript source persistence
  - distill job execution
  - artifact persistence
  - optional memory-row persistence
- plugin shell adds management-gated entrypoints for:
  - distill enqueue
  - distill status polling

Expected module ownership:

- typed DTOs and transport: `src/backend-client/types.ts`, `src/backend-client/client.ts`
- tool surface and validation: `src/backend-tools.ts`
- runtime/docs alignment: `README.md`, `README_CN.md`, `openclaw.plugin.json`

### Backend recall debug trace surfaced through the shell

Selected shell behavior:

- ordinary recall remains on:
  - `POST /v1/recall/generic`
  - `POST /v1/recall/reflection`
- debug trace remains on:
  - `POST /v1/debug/recall/generic`
  - `POST /v1/debug/recall/reflection`
- plugin shell exposes debug trace only through an explicit management/debug path, not through ordinary recall output.

### `memoryReflection` config and local reflection residue

Selected boundary:

- `/new` and `/reset` stay as enqueue-only adapter behavior;
- local reflection generation helpers are not part of the supported runtime;
- stale local-generation config fields `agentId`, `maxInputChars`, `timeoutMs`, and `thinkLevel` remain parseable-but-ignored during this scope;
- schema/docs/startup diagnostics must label those fields as deprecated/ignored compatibility fields, not supported runtime controls.

### Prompt-local `setwise-v2`

Selected boundary:

- `setwise-v2` remains a prompt-local post-selection seam over already-returned backend rows;
- it must not require ordinary runtime recall DTO expansion;
- its supported runtime inputs are the fields actually available after backend recall mapping:
  - `id`
  - `text`
  - `score`
  - `category`
  - `scope`
  - timestamps

## Operational behavior

### Startup and config parse

- `remoteBackend.enabled=true` remains mandatory.
- startup must not initialize any local memory authority modules.
- if stale `memoryReflection` local-generation keys remain accepted, startup/logging must make their ignored/deprecated status explicit.

### Runtime modes after this scope

- ordinary memory tools:
  - unchanged behavior and current fail-open/fail-closed rules.
- management tools:
  - are frozen as distill enqueue/status and debug recall trace;
  - must require runtime principal identity and remain caller-scoped.
- prompt-time auto-recall:
  - continues to use backend generic recall;
  - local `setwise-v2` only trims already-returned rows under the stable ordinary DTO.
- reflection hooks:
  - `/new` and `/reset` remain enqueue-only;
  - no local reflection generation fallback is part of the supported runtime.

## Observability and error handling

- ordinary recall continues to fail open when runtime principal identity or backend availability prevents prompt injection;
- write/update/delete/list/stats/distill/debug management surfaces fail closed with explicit errors when runtime principal identity is missing;
- distill surfaces must return job ids, status, and structured backend errors without inventing local job state;
- debug recall surfaces must return trace payloads only on explicit operator requests and only under the caller principal boundary;
- README and tool descriptions must clearly separate:
  - ordinary runtime recall
  - debug trace inspection
  - distill job management

## Security model and hardening notes

- adapter must continue to forward trusted `userId` and `agentId` only from runtime context; no static fallback principal synthesis;
- distill and debug management tools must not accept client-provided scope overrides;
- trace surfaces must not leak another principal's data or bypass existing runtime auth requirements;
- any returned trace or distill diagnostic data must be bounded and safe for tool output rendering;
- removing dead local reflection code must not accidentally remove redaction or transcript filtering used by still-supported paths.

## Phase 1 baseline verification matrix

Node/plugin shell baseline:

- `npm test`
  - full repo baseline before runtime edits
- `node --test test/remote-backend-shell-integration.test.mjs`
  - shell/adapter contract baseline
- `node --test test/backend-client-retry-idempotency.test.mjs`
  - backend-client retry and transport assumptions
- `node --test test/memory-reflection.test.mjs`
  - reflection compatibility baseline before local cleanup
- `node --test test/config-session-strategy-migration.test.mjs`
  - config migration/deprecation baseline

Backend contract guard:

- `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
  - required whenever adapter DTO expectations or backend-facing assumptions are touched

Doc/refactor hygiene:

- `bash /root/.codex/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/adapter-surface-closeout-2026-03-17`
  - ensure scope docs have no scaffold/template residue
- `bash /root/.codex/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/adapter-surface-closeout-2026-03-17 README.md`
  - ensure no stale text residue remains after the freeze/update batch
