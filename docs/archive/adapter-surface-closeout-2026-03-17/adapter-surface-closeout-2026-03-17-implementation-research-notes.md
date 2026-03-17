---
description: Implementation research notes for adapter-surface-closeout-2026-03-17.
---

# adapter-surface-closeout-2026-03-17 Implementation Research Notes

## Problem statement and current baseline

Current canonical runtime story in docs:

- `docs/runtime-architecture.md` says `backend/` owns memory authority, `index.ts` + `src/backend-client/*` + `src/backend-tools.ts` are the adapter layer, and `src/context/*` is prompt-time orchestration only.
- `README.md` repeats that split and claims support for backend-native `distill`, debug recall routes, and a remote-only runtime.

Scan-time runtime wiring in code (pre-Phase 2/3 implementation):

- `index.ts` boots only the remote backend client and local orchestration seams; `remoteBackend.enabled=true` is mandatory.
- `src/backend-tools.ts` registers only `memory_recall`, `memory_store`, `memory_forget`, `memory_update`, and optional `memory_list` / `memory_stats`, plus self-improvement tools.
- `src/backend-client/client.ts` already implements:
  - `enqueueDistillJob`
  - `getDistillJobStatus`
  - transcript append
  - generic/reflection recall
  - write/update/delete/list/stats
- `backend/src/lib.rs` already exposes:
  - `POST /v1/debug/recall/generic`
  - `POST /v1/debug/recall/reflection`
  - `POST /v1/distill/jobs`
  - `GET /v1/distill/jobs/{jobId}`
- `test/remote-backend-shell-integration.test.mjs` covers transcript append and ordinary remote shell paths, but not a public distill/debug tool surface.

Residual local/runtime code still present at scan time:

- `index.ts` still contains the old embedded/CLI reflection-generation stack (`loadEmbeddedPiRunner`, `runReflectionViaCli`, `buildReflectionPrompt`, `generateReflectionText`), even though `/new` and `/reset` now read session content and call `enqueueReflectionJob`.
- `openclaw.plugin.json` still exposes `memoryReflection.agentId`, `maxInputChars`, `timeoutMs`, and `thinkLevel`, which were relevant to local reflection generation, not the current enqueue-only path.
- `src/context/auto-recall-orchestrator.ts` maps backend rows into local `RecallResultRow` objects with `vector: []` and `sources: {}`.
- `src/prompt-local-topk-setwise-selection.ts` still supports embedding-driven semantic penalties, but ordinary runtime recall rows no longer provide embeddings.
- top-level `src/reflection-store.ts` still exports local LanceDB-era helpers such as `storeReflectionToLanceDB`, and `test/memory-reflection.test.mjs` still imports them as reference behavior.
- `src/query-expander.ts` and `src/chunker.ts` remain in the repo, but no active runtime import path was found.
- `package.json` still advertises `lancedb`, `bm25`, `hybrid-retrieval`, `rerank`, and `chunking`, and still keeps `@lancedb/lancedb` and `openai` as dependencies.

## Gap analysis with evidence

1. **Distill backend parity exists, but plugin shell parity is incomplete.**
   Evidence:
   - `README.md` lists `Distill job enqueue + polling` and `session-transcript` distill as supported.
   - `src/backend-client/client.ts` already provides typed distill calls.
   - `backend/src/lib.rs` and `backend/tests/phase2_contract_semantics.rs` already implement and test the backend contract.
   - `src/backend-tools.ts` does not expose any distill tool or status surface.

2. **Recall debug trace parity exists in Rust, but the adapter layer does not surface it.**
   Evidence:
   - `README.md` documents `/v1/debug/recall/*`.
   - `backend/src/lib.rs` exposes both debug recall routes.
   - `src/backend-client/types.ts` and `src/backend-client/client.ts` do not model those routes.
   - the plugin has no management/debug tool that can retrieve structured recall traces.

3. **`memoryReflection` still exposes dead local-generation knobs.**
   Evidence:
   - `openclaw.plugin.json` still documents `agentId`, `maxInputChars`, `timeoutMs`, and `thinkLevel`.
   - `index.ts` still contains the local reflection-generation helper stack.
   - active `/new` and `/reset` runtime flow in `index.ts` only assembles transcript items and enqueues backend reflection jobs.

4. **`setwise-v2` is half-retained: intentionally local, but wired to missing production signals.**
   Evidence:
   - `README.md` explicitly keeps `setwise-v2` as a prompt-local seam.
   - `src/context/auto-recall-orchestrator.ts` calls `selectPromptLocalAutoRecallResults`.
   - the mapping from backend rows clears embeddings and source flags.
   - the local setwise selector still contains semantic-embedding and source-aware logic that ordinary runtime rows never exercise.

5. **Residual TS/package debt still overstates the local stack.**
   Evidence:
   - `README.md` only enumerates a subset of retained TypeScript seams, but top-level `src/reflection-store.ts` and related helpers remain.
   - `test/memory-reflection.test.mjs` still imports those helpers directly.
   - `query-expander.ts`, `chunker.ts`, and old package metadata continue to project a local RAG identity that the runtime no longer uses.

## Architecture/implementation options and trade-offs

### Option 1: Docs-only correction

- change README/docs/schema descriptions;
- do not ship distill/debug shell surfaces;
- leave dead local runtime helpers in place.

Trade-offs:

- low code churn;
- leaves the adapter incomplete;
- keeps residual maintenance confusion.

### Option 2: Narrow shell-surface completion with compatibility-preserving cleanup

- add management-gated distill enqueue/status tooling;
- add management-gated recall debug trace tooling;
- either remove or explicitly deprecate stale local reflection-generation helpers and config fields;
- redefine prompt-local `setwise-v2` around stable runtime row fields only;
- remove or relocate residual unused TS/package debt after import-proof.

Trade-offs:

- directly addresses the concrete scan findings;
- keeps backend authority and stable ordinary DTOs intact;
- requires careful staging across tools, config, docs, and tests.

### Option 3: Broader shell architecture rewrite

- redesign the plugin public surface and orchestration boundaries together;
- revisit whether retained prompt-local seams should survive.

Trade-offs:

- higher architectural ambition;
- poor fit for the immediate gaps;
- large rollback and review burden.

## Selected design and rationale

Select **Option 2**.

Design summary:

1. **Complete the adapter surface only where the backend contract already exists.**
   - add backend-client DTOs and methods for debug recall trace routes;
   - expose management-gated plugin tools for:
     - `memory_distill_enqueue`
     - `memory_distill_status`
     - `memory_recall_debug`
   - keep debug recall trace on a dedicated management tool instead of extending ordinary `memory_recall` with a debug flag.

2. **Keep ordinary runtime DTOs stable.**
   - do not add embeddings or detailed score internals to ordinary `/v1/recall/*`;
   - keep trace payloads on explicit debug paths only.

3. **Make stale local reflection config an explicit compatibility story instead of silent drift.**
   - selected implementation for this scope:
     - remove dead local reflection-generation runtime code from `index.ts`;
     - keep deprecated config fields parseable for one compatibility release;
     - mark `agentId`, `maxInputChars`, `timeoutMs`, and `thinkLevel` ignored/deprecated in schema/docs and startup diagnostics;
     - do not let those fields affect runtime behavior.

4. **Reclassify `setwise-v2` as a stable prompt-local lexical/coverage seam.**
   - production semantics should rely only on data actually available in ordinary recall rows:
     - `id`
     - `text`
     - `score`
     - `category`
     - `scope`
     - timestamps
   - embedding-only semantic penalties should be removed from the production contract or made explicitly optional/test-only.

5. **Finish the residual-debt cleanup only after import/use-site proof.**
   - target `src/reflection-store.ts`, `src/query-expander.ts`, `src/chunker.ts`, and residual `package.json` metadata for removal, relocation, or explicit non-runtime classification only after import/use-site proof;
   - align `package.json`, README, and tests with the actual remote-authority runtime story.

## Phase 2/3 implementation outcome

- `src/backend-client/types.ts` and `src/backend-client/client.ts` now expose typed debug recall DTOs and transport for `/v1/debug/recall/generic` and `/v1/debug/recall/reflection`.
- `src/backend-tools.ts` now exposes:
  - `memory_distill_enqueue`
  - `memory_distill_status`
  - `memory_recall_debug`
- those new management surfaces remain gated behind `enableManagementTools=true` and continue to fail closed on missing runtime principal identity.
- `index.ts` no longer retains the old embedded/CLI reflection-generation stack; supported runtime behavior stays enqueue-only for `/new` and `/reset`.
- `parsePluginConfig` and `openclaw.plugin.json` now treat `memoryReflection.agentId`, `maxInputChars`, `timeoutMs`, and `thinkLevel` as deprecated/ignored compatibility fields, and plugin startup logs when they are still configured.
- prompt-local `setwise-v2` no longer depends on ordinary-row embeddings or source breakdown metadata in the supported runtime path; current tests document lexical/coverage behavior over ordinary backend rows.

## Phase 4 implementation outcome

- import/use-site proof closed the remaining local-RAG residue:
  - `src/chunker.ts` had no active runtime or test/reference import path and has been removed;
  - `package.json` / `package-lock.json` no longer advertise or depend on `@lancedb/lancedb` or `openai`;
  - `src/query-expander.ts` remains only because `test/query-expander.test.mjs` imports it as a lexical reference helper;
  - `src/reflection-store.ts` remains only because `test/memory-reflection.test.mjs` imports its reference helpers.
- `README.md`, `README_CN.md`, and `openclaw.plugin.json` now align with the shipped surface:
  - they enumerate `memory_distill_enqueue`, `memory_distill_status`, and `memory_recall_debug` as management-gated tools;
  - they describe `setwise-v2` as a prompt-local lexical/coverage selector over backend-returned rows, not as a second backend ranking path;
  - they mark `memoryReflection.agentId`, `maxInputChars`, `timeoutMs`, and `thinkLevel` as parseable-but-ignored compatibility fields.
- verification for the closeout batch passed via:
  - `npm test`
  - `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
  - `bash /root/.codex/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/adapter-surface-closeout-2026-03-17`
  - `bash /root/.codex/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/adapter-surface-closeout-2026-03-17 README.md`
- Phase 4 release guardrails are now explicit:
  - no local fallback authority or anonymous management/debug path is reintroduced;
  - `src/query-expander.ts` and `src/reflection-store.ts` are test/reference-only seams and must not be imported into supported runtime modules without reopening the architecture docs;
  - the deprecated `memoryReflection.*` compatibility fields should only be removed in a later breaking-change window once downstream configs are migrated.

## Test and validation strategy

Primary repo commands:

- `npm test`
- `bash /root/.codex/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/adapter-surface-closeout-2026-03-17`
- `bash /root/.codex/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/adapter-surface-closeout-2026-03-17 README.md`

Focused Node checks expected during implementation:

- `node --test test/remote-backend-shell-integration.test.mjs`
- `node --test test/backend-client-retry-idempotency.test.mjs`
- `node --test test/memory-reflection.test.mjs`
- `node --test test/config-session-strategy-migration.test.mjs`

Backend contract checks required when the scope touches backend-facing DTOs or assumptions:

- `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`

Expected outcomes:

- plugin management surfaces exist for distill and recall debug trace, or docs are explicitly narrowed with no overclaim left;
- stale `memoryReflection` local-generation knobs are either removed from supported config or clearly documented as ignored/deprecated;
- `setwise-v2` behavior is test-backed against the actual runtime row shape;
- unused local RAG/package residue is either deleted or explicitly classified as non-runtime.

## Risks, assumptions, and freeze outcome

Risks:

- distill/debug tool design may overexpose internal/backend detail if returned content is not bounded;
- deleting local reflection helpers can unexpectedly break tests or bootstrap paths if references were missed;
- dependency cleanup can produce incidental packaging churn.

Assumptions:

- backend debug recall and distill contracts are stable enough to be surfaced without changing Rust routes;
- management/debug surfaces may be gated under `enableManagementTools` without violating current operator expectations;
- ordinary recall rows will remain intentionally narrow.

Phase 1 resolved decisions:

- public management tool names are frozen as:
  - `memory_distill_enqueue`
  - `memory_distill_status`
  - `memory_recall_debug`
- debug recall remains a dedicated management surface and is not folded into ordinary `memory_recall`;
- deprecated `memoryReflection.agentId`, `maxInputChars`, `timeoutMs`, and `thinkLevel` remain parseable-but-ignored for this scope;
- `query-expander.ts`, `chunker.ts`, `src/reflection-store.ts`, and residual `package.json` local-RAG metadata are frozen as cleanup targets pending import/use-site proof in later phases.

Non-blocking follow-up questions:

- whether startup diagnostics should be emitted once per process or once per deprecated field during the compatibility window;
- whether the existing dead-code warning for `RateLimited` in `backend/src/error.rs` should be cleaned up in a later backend hygiene scope.
