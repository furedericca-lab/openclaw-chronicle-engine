---
description: Execution and audit checklist for remote-authority-reset with implementation-ready remote-only deletion stages.
---

# Phases Checklist: remote-authority-reset

## Input
- `docs/remote-authority-reset/remote-authority-reset-brainstorming.md`
- `docs/remote-authority-reset/remote-authority-reset-implementation-research-notes.md`
- `docs/remote-authority-reset/remote-authority-reset-scope-milestones.md`
- `docs/archive/remote-authority-reset/remote-authority-reset-technical-documentation.md`
- `docs/remote-authority-reset/remote-authority-reset-contracts.md`
- `docs/remote-authority-reset/remote-only-local-authority-removal-plan.md`
- `docs/remote-authority-reset/task-plans/phase-1-remote-authority-reset.md`
- `docs/remote-authority-reset/task-plans/phase-2-remote-authority-reset.md`
- `docs/remote-authority-reset/task-plans/phase-3-remote-authority-reset.md`
- `docs/remote-authority-reset/task-plans/phase-4-remote-authority-reset.md`

## Rules
- This file is the execution and audit hub.
- Do not mark a phase complete without recorded verification output.
- Keep archive documents historical; keep canonical docs authoritative.
- Follow staged rollback boundaries from the remote-only removal plan.

## Global Status Board
| Phase | Status | Completion | Health | Blockers |
|---|---|---|---|---|
| 1 | Completed | 100% | Green | 0 |
| 2 | Completed | 100% | Green | 0 |
| 3 | Completed | 100% | Green | 0 |
| 4 | Completed | 100% | Green | 0 |

## Phase Entry Links
1. [phase-1-remote-authority-reset.md](./phase-1-remote-authority-reset.md)
2. [phase-2-remote-authority-reset.md](./phase-2-remote-authority-reset.md)
3. [phase-3-remote-authority-reset.md](./phase-3-remote-authority-reset.md)
4. [phase-4-remote-authority-reset.md](./phase-4-remote-authority-reset.md)

## Planning Batch Record (2026-03-15)

This batch delivered planning-layer completion for remote-only cleanup/removal.

Delivered:
- remote-only deletion runbook upgraded with exact symbol-level entry points, temporary migration branches, permanent keep surfaces, staged gates, and rollback rules.
- phase plans (2/3/4) rewritten from generic migration language into executable deletion-stage tasks.
- canonical docs aligned so milestones/contracts/technical notes reference hard remote-only enforcement and staged local-authority deletion.

Not executed in this planning batch:
- broad code deletion of local-authority modules.
- phase 2/3/4 implementation tasks.

## Execution Batch Record (2026-03-15, remote-only cleanup #1)

This batch started implementation (not just planning) with safe local-authority isolation:

Delivered:
- `index.ts`
  - added explicit runtime warning when legacy local-authority migration mode is active.
  - disabled legacy `memory-pro` CLI registration by default in local migration mode.
  - kept migration-only local CLI behind explicit env opt-in: `MEMORY_LANCEDB_PRO_ENABLE_LEGACY_LOCAL_CLI=1`.
- `openclaw.plugin.json`
  - reduced promotion of local-authority config surfaces in descriptions/uiHints (`embedding.*`, `dbPath`, `memoryReflection.storeToLanceDB`, `mdMirror.*`) by marking them as legacy-deprecated migration fields.
  - clarified `remoteBackend` as canonical supported authority mode.
- `README.md` and `README_CN.md`
  - aligned CLI wording to disabled-by-default legacy behavior and explicit migration-only env opt-in.
- tests
  - `test/remote-backend-shell-integration.test.mjs`: added coverage for local legacy CLI disabled-by-default and explicit env opt-in behavior.
  - `test/config-session-strategy-migration.test.mjs`: switched baseline config to remote-authority mode; kept one explicit legacy-local embedding compatibility case.

Verification evidence (executed):
- `node --test test/remote-backend-shell-integration.test.mjs` -> pass (`20/20`).
- `node --test test/memory-reflection.test.mjs` -> pass (`64/64`).
- `node --test test/config-session-strategy-migration.test.mjs` -> pass (`9/9`).
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/remote-authority-reset` -> `[OK] placeholder scan clean`.
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/remote-authority-reset README.md` -> `[OK] ... passed`.
- `git diff --check` -> clean.
- `npm test` -> pass (`175/175` + `cli-smoke` pass).

Deferred local-authority surfaces after this batch:
- full removal of local runtime implementation modules (`src/store.ts`, `src/retriever.ts`, `src/embedder.ts`, `src/tools.ts`, `src/migrate.ts`, `cli.ts`, `src/scopes.ts`, `src/access-tracker.ts`).
- removal of `index.ts` local runtime initialization and local reflection persistence branches.
- hard parse-time rejection of `remoteBackend.enabled=false` (currently still migration-compatible).

## Execution Batch Record (2026-03-15, remote-only cleanup #2)

This batch executed hard remote-only cutover changes in runtime, schema, and first removable local-authority files/tests.

Delivered:
- `index.ts`
  - enforced hard parse contract: `remoteBackend.enabled=true` is mandatory; local embedding/local-authority parse path deleted.
  - removed local runtime wiring branches: local tools, local CLI, local auto-capture, local reflection execution/persistence, local startup checks, local backup path.
  - kept remote adapter + context-engine orchestration hooks only.
- `openclaw.plugin.json`
  - removed active local-authority schema surfaces: `embedding`, `dbPath`, `retrieval`, `scopes`, `mdMirror`, and `memoryReflection.storeToLanceDB`.
  - set `remoteBackend` as required and enforced `remoteBackend.enabled: true` via schema.
  - removed `autoRecallSelectionMode=legacy` enum alias.
- deleted local-authority module/test surfaces
  - deleted: `src/tools.ts`, `src/migrate.ts`, `cli.ts`.
  - deleted tests: `test/cli-smoke.mjs`, `test/migrate-legacy-schema.test.mjs`.
  - rewrote `test/memory-reflection.test.mjs` to remove local runtime monkeypatch suites (`MemoryStore.prototype.*`, `MemoryRetriever.prototype.*`).
- updated runtime/docs metadata
  - `package.json` test script no longer references deleted local CLI/migrator tests.
  - `README.md` / `README_CN.md` no longer describe `cli.ts`/`memory-pro` local runtime paths as available.

Deferred local-authority surfaces after this batch:
- local authority implementation modules still present by design in this stage: `src/store.ts`, `src/retriever.ts`, `src/embedder.ts`, `src/scopes.ts`, `src/access-tracker.ts`.
- transitive type coupling remains in context/reflection modules that still import local types (`src/context/*`, `src/reflection-store.ts`, `src/reflection-recall.ts`, `src/auto-recall-final-selection.ts`).
- local-only test suites coupled to those modules remain until type extraction + module deletion stage.

## Execution Batch Record (2026-03-16, remote-only cleanup #3)

This batch completed Phase 3 hard deletion for the remaining local-authority coupling chain.

Delivered:
- deleted remaining local-authority modules:
  - `src/store.ts`
  - `src/retriever.ts`
  - `src/embedder.ts`
  - `src/scopes.ts`
  - `src/access-tracker.ts`
- deleted remaining local-coupled benchmark/runtime surfaces:
  - `src/benchmark.ts`
  - `test/benchmark-runner.mjs`
- deleted local-only test suites coupled to removed modules:
  - `test/access-tracker.test.mjs`
  - `test/retriever-trace.test.mjs`
  - `test/vector-search-cosine.test.mjs`
  - `test/embedder-error-hints.test.mjs`
  - `test/ollama-no-apikey.test.mjs`
  - `test/vllm-provider.test.mjs`
- removed transitive type coupling in permanent modules:
  - added `src/memory-record-types.ts` (remote-safe shared entry/recall row types).
  - rewired `src/context/auto-recall-orchestrator.ts`, `src/context/reflection-prompt-planner.ts`, `src/reflection-store.ts`, `src/reflection-recall.ts`, `src/auto-recall-final-selection.ts`.
  - removed local fallback planner dependencies (`retrieve`, `storeList`, `getAccessibleScopes`) from permanent planner seams.
- test/docs/runtime updates:
  - rewrote planner coverage in `test/memory-reflection.test.mjs` to remote-only dependency stubs.
  - updated `package.json` test script to remove deleted local suites.

Verification evidence (executed):
- `for f in src/store.ts src/retriever.ts src/embedder.ts src/scopes.ts src/access-tracker.ts; do [ ! -e "$f" ] || echo "still present: $f"; done` -> no output.
- `node --test test/remote-backend-shell-integration.test.mjs test/memory-reflection.test.mjs test/backend-client-retry-idempotency.test.mjs` -> pass (`78/78`).
- `npm test` -> pass (`92/92`).
- `git diff --check` -> clean.
- broad residual grep (`store|retriever|embedder|...`) returns only substring false positives (`reflection-store.js`, `backend-tools.js`, `self-improvement-tools.js`), not deleted-module import edges.
- strict import-edge grep for deleted local modules and `cli.js` -> no matches.

## Execution Batch Record (2026-03-16, remote-only cleanup #4 / Phase 4 closeout)

This batch completed release-ready closeout for the remote-authority-reset track.

Delivered:
- residual wording drift cleanup in active user-facing surfaces:
  - `README.md`
  - `README_CN.md`
  - `openclaw.plugin.json` (`remoteBackend.enabled` ui hint no longer implies local embedding config paths)
  - `docs/remote-authority-reset/README.md`
  - `docs/archive/remote-authority-reset/remote-authority-reset-technical-documentation.md`
- added release-cut/rollback notes:
  - `docs/remote-authority-reset/phase-4-closeout-release-notes.md`
- re-confirmed remote principal + backend-owned scope invariants in active code/docs evidence:
  - `src/backend-client/runtime-context.ts`
  - `src/backend-tools.ts`
  - `index.ts`

Verification evidence (executed):
- `npm test` -> pass (`92/92`).
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/remote-authority-reset` -> `[OK] placeholder scan clean`.
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/remote-authority-reset README.md` -> all checks `[OK]`.
- `find docs/archive/2026-03-15-architecture-reset -maxdepth 2 -type f | sort` -> archive document set present and unchanged.
- `find docs/remote-authority-reset -maxdepth 2 -type f | sort` -> canonical set includes `phase-4-closeout-release-notes.md`.
- `git diff --check` -> clean.
- `rg -n "MissingRuntimePrincipalError|missing_runtime_principal|buildToolCallContext\\(|scope authority is backend-owned|backend-owned scope semantics" src/backend-tools.ts src/backend-client/runtime-context.ts` -> confirms principal enforcement path and backend-owned scope tool semantics.
- `rg -n "remoteBackend\\.enabled=true is required|enqueueReflectionJob|command:new|command:reset|enqueue blocked \\(missing runtime principal" index.ts` -> confirms remote-only parse contract and remote reflection enqueue flow.

## Phase Execution Records

### Phase 1 — Documentation reset and architecture freeze
- Completion checklist:
  - [x] Archive old active docs into `docs/archive/2026-03-15-architecture-reset/`.
  - [x] Create canonical docs under `docs/remote-authority-reset/`.
  - [x] Establish one canonical architecture statement.
- Verification references:
  - `find docs/archive/2026-03-15-architecture-reset -maxdepth 2 -type f | sort`
  - `find docs/remote-authority-reset -maxdepth 2 -type f | sort`
- Checkpoint confirmed:
  - Canonical architecture scope established.

### Phase 2 — Hard remote-only enforcement (implementation)
- Completion checklist:
  - [x] Enforce remote-only parse/runtime contract in `index.ts`.
  - [x] Remove local runtime branches in `index.ts` that remain from migration mode.
  - [x] Prune local-authority schema/help fields in `openclaw.plugin.json`.
  - [x] Rewrite and pass remote-only config contract tests for this batch scope.
- Required verification:
  - `node --test test/remote-backend-shell-integration.test.mjs`
  - `node --test test/config-session-strategy-migration.test.mjs`
  - `rg -n "registerAllMemoryTools|createMemoryCLI|localRuntime|createScopeManager|createMigrator|AccessTracker" index.ts`
  - `rg -n "\"embedding\"|\"dbPath\"|\"retrieval\"|\"scopes\"|\"mdMirror\"|storeToLanceDB" openclaw.plugin.json`
- Evidence (2026-03-15 cleanup #2):
  - `parsePluginConfig` now hard-fails when `remoteBackend.enabled !== true`.
  - `index.ts` no longer contains `localRuntime`/local tool or CLI branches.
  - schema no longer exposes local-authority runtime fields.
  - remote-contract tests pass with updated assertions.
- Checkpoint target:
  - Runtime is remote-only enforced and ready for physical module deletions.

### Phase 3 — Local-authority module deletion + test rewrite (implementation)
- Completion checklist:
  - [x] Delete local-authority module set listed in the removal plan.
  - [x] Remove type-coupling imports from permanent modules.
  - [x] Rewrite/remove local-authority test suites.
  - [x] Verify no runtime/test import references deleted modules.
- Evidence (2026-03-15 cleanup #2):
  - deleted: `src/tools.ts`, `src/migrate.ts`, `cli.ts`.
  - deleted: `test/cli-smoke.mjs`, `test/migrate-legacy-schema.test.mjs`.
  - rewritten: `test/memory-reflection.test.mjs` (local class monkeypatch suites removed).
- Evidence (2026-03-16 cleanup #3):
  - deleted: `src/store.ts`, `src/retriever.ts`, `src/embedder.ts`, `src/scopes.ts`, `src/access-tracker.ts`, `src/benchmark.ts`.
  - deleted tests: `test/access-tracker.test.mjs`, `test/retriever-trace.test.mjs`, `test/vector-search-cosine.test.mjs`, `test/embedder-error-hints.test.mjs`, `test/ollama-no-apikey.test.mjs`, `test/vllm-provider.test.mjs`, `test/benchmark-runner.mjs`.
  - type uncoupling landed in `src/context/*`, `src/reflection-store.ts`, `src/reflection-recall.ts`, `src/auto-recall-final-selection.ts` via `src/memory-record-types.ts`.
- Required verification:
  - `for f in src/store.ts src/retriever.ts src/embedder.ts src/tools.ts src/migrate.ts src/scopes.ts src/access-tracker.ts cli.ts; do [ ! -e "$f" ] || echo "still present: $f"; done`
  - `rg -n "store\.js|retriever\.js|embedder\.js|migrate\.js|tools\.js|scopes\.js|access-tracker\.js|cli\.js" index.ts src test`
  - `node --test test/remote-backend-shell-integration.test.mjs test/memory-reflection.test.mjs test/backend-client-retry-idempotency.test.mjs`
- Checkpoint target:
  - Source tree contains no executable local-authority implementation path.

### Phase 4 — Final verification and release closeout (implementation)
- Completion checklist:
  - [x] Run full regression set after deletions.
  - [x] Clean remaining wording drift across README/schema/docs.
  - [x] Run doc hygiene and archive sanity checks.
  - [x] Re-confirm remote principal / backend-owned scope invariants.
  - [x] Publish rollback-ready release notes.
- Required verification:
  - `npm test`
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/remote-authority-reset`
  - `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/remote-authority-reset README.md`
  - `find docs/archive/2026-03-15-architecture-reset -maxdepth 2 -type f | sort`
  - `find docs/remote-authority-reset -maxdepth 2 -type f | sort`
  - `git diff --check`
  - `rg -n "MissingRuntimePrincipalError|missing_runtime_principal|buildToolCallContext\\(|scope authority is backend-owned|backend-owned scope semantics" src/backend-tools.ts src/backend-client/runtime-context.ts`
  - `rg -n "remoteBackend\\.enabled=true is required|enqueueReflectionJob|command:new|command:reset|enqueue blocked \\(missing runtime principal" index.ts`
- Evidence (2026-03-16 cleanup #4):
  - `npm test` -> pass (`92/92`).
  - doc placeholder + post-refactor scans -> pass.
  - archive/canonical path inspections -> pass; canonical tree includes `docs/remote-authority-reset/phase-4-closeout-release-notes.md`.
  - `git diff --check` -> clean.
  - invariant grep checks -> expected matches present for principal enforcement, backend-owned scope wording, remote-only parse error contract, and remote reflection enqueue path.
- Release note:
  - `docs/remote-authority-reset/phase-4-closeout-release-notes.md`
- Closeout statement:
  - Phase 4 closeout is complete in this worktree with verification evidence recorded.
- Checkpoint target:
  - Remote-only architecture is release-auditable and rollback-safe.

## Final Release Gate
- One runtime authority model only: remote backend authority.
- Adapter remains transport/integration-only; context-engine remains prompt-time-only.
- No local-authority runtime code/config/test paths remain.
- Verification evidence is recorded for every completed phase.
