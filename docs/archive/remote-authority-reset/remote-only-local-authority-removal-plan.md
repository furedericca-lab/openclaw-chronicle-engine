---
description: Implementation-ready remote-only cleanup runbook for deleting legacy local-authority code paths.
---

# remote-authority-reset Remote-Only Local-Authority Removal Plan

## Objective

Make remote backend authority the only runtime path in code, config schema, and tests.

Target end state:
- `remoteBackend.enabled=true` is mandatory for supported runtime.
- `index.ts` keeps only adapter + context-engine wiring.
- local LanceDB authority modules and CLI/migration surfaces are deleted.
- test suite no longer encodes local-authority runtime compatibility.

This document is the execution source of truth for deletion work.

## Baseline Snapshot (Before Deletion)

Snapshot date: 2026-03-15 (`dev/context-engine-split`)

Remote-only support already exists, but local-authority runtime branches still exist as deprecated migration paths.

## Execution Status Update (2026-03-15, cleanup #2)

Implemented in this batch:
- hard remote-only parse contract landed (`remoteBackend.enabled=true` required).
- `index.ts` local runtime execution branches removed (local tools/CLI/auto-capture/reflection/startup/backup paths).
- schema cutover landed in `openclaw.plugin.json`:
  - removed active local-authority config fields (`embedding`, `dbPath`, `retrieval`, `scopes`, `mdMirror`, `memoryReflection.storeToLanceDB`);
  - `remoteBackend` is required with `enabled: true` enforced;
  - removed `autoRecallSelectionMode: legacy` alias.
- deleted local-authority files/tests:
  - `src/tools.ts`, `src/migrate.ts`, `cli.ts`;
  - `test/cli-smoke.mjs`, `test/migrate-legacy-schema.test.mjs`.
- rewrote `test/memory-reflection.test.mjs` to remove local runtime monkeypatch suites.

Still deferred:
- module deletions requiring type untangling:
  - `src/store.ts`, `src/retriever.ts`, `src/embedder.ts`, `src/scopes.ts`, `src/access-tracker.ts`.
- transitive type coupling in context/reflection modules:
  - `src/context/auto-recall-orchestrator.ts`
  - `src/context/reflection-prompt-planner.ts`
  - `src/reflection-store.ts`
  - `src/reflection-recall.ts`
  - `src/auto-recall-final-selection.ts`

## Execution Status Update (2026-03-16, cleanup #3)

Implemented in this batch:
- completed remaining local-authority module deletions:
  - `src/store.ts`, `src/retriever.ts`, `src/embedder.ts`, `src/scopes.ts`, `src/access-tracker.ts`.
- completed local benchmark/runtime cleanup coupled to removed retriever/store chain:
  - `src/benchmark.ts`, `test/benchmark-runner.mjs`.
- completed local-only test deletion set for removed modules:
  - `test/access-tracker.test.mjs`
  - `test/retriever-trace.test.mjs`
  - `test/vector-search-cosine.test.mjs`
  - `test/embedder-error-hints.test.mjs`
  - `test/ollama-no-apikey.test.mjs`
  - `test/vllm-provider.test.mjs`
- completed transitive type uncoupling in permanent modules:
  - added `src/memory-record-types.ts`.
  - rewired `src/context/auto-recall-orchestrator.ts`, `src/context/reflection-prompt-planner.ts`, `src/reflection-store.ts`, `src/reflection-recall.ts`, `src/auto-recall-final-selection.ts`.
  - removed local fallback planner dependencies (`retrieve`, `storeList`, `getAccessibleScopes`) from permanent context planners.

Post-batch state:
- `R3` stage gate is satisfied for local-authority module deletion and import-graph convergence.
- local-authority runtime files listed in this plan are physically removed from `src/`.

## Exact Legacy Entry Points To Remove

### 1) Top-level imports and local runtime wiring in `index.ts`

Local-authority imports currently present:
- `import { MemoryStore, validateStoragePath } from "./src/store.js";`
- `import { createEmbedder, getVectorDimensions } from "./src/embedder.js";`
- `import { createRetriever, DEFAULT_RETRIEVAL_CONFIG } from "./src/retriever.js";`
- `import { createScopeManager } from "./src/scopes.js";`
- `import { createMigrator } from "./src/migrate.js";`
- `import { registerAllMemoryTools } from "./src/tools.js";`
- `import type { MdMirrorWriter } from "./src/tools.js";`
- `import { AccessTracker } from "./src/access-tracker.js";`
- `import { createMemoryCLI } from "./cli.js";`

All imports above are deletion targets except pieces explicitly re-homed to permanent adapter/context surfaces.

### 2) Temporary migration-only runtime branches in `index.ts`

Delete these branches in remote-only cutover:
- local runtime initializer (`const localRuntime = !remoteBackendEnabled ? ... : null`) including:
  - local db path validation,
  - local embedder/retriever/store init,
  - `scopeManager` / `migrator` / `accessTracker` init.
- local tool registration branch:
  - `registerAllMemoryTools(...)` path.
- local CLI registration branch:
  - `api.registerCli(createMemoryCLI(...), { commands: ["memory-pro"] })` path.
- local auto-recall dependency branch:
  - planner dependencies `retrieve` + `getAccessibleScopes` local fallback.
- local auto-capture branch:
  - local dedupe + `store.store(...)` + optional `mdMirror` write path.
- reflection planner local fallback dependencies:
  - `storeList` + `getAccessibleScopes` local path.
- local reflection `/new` + `/reset` execution branch:
  - inline reflection generation,
  - local mapped-memory writes,
  - `storeReflectionToLanceDB(...)` local persistence path,
  - local derived-focus reconstruction from local store.
- local service startup/shutdown branch:
  - local startup checks (`embedder.test()`, `retriever.test()`),
  - local periodic backup,
  - `accessTracker.flush()/destroy()` stop path.

### 3) Temporary migration-only config parsing branches in `index.ts`

Delete or replace with remote-only hard enforcement:
- `if (!remoteBackendEnabled) { ... embedding config required ... }` parse block.
- local fallback key behavior for embedding (`OPENAI_API_KEY`/`"no-key-required"`).
- local-mode parse error text:
  - `embedding config is required when remoteBackend is disabled ...`.
- legacy compatibility aliases that only served local paths:
  - `sessionMemory.enabled` mapping branch,
  - `autoRecallSelectionMode: "legacy" -> "mmr"` alias.

### 4) Local tool/CLI modules to delete

Primary deletions:
- `src/tools.ts`
- `cli.ts`
- `src/migrate.ts`

Exact migration-only tool branches in `src/tools.ts`:
- `registerMemoryRecallTool`: local `scope` parameter + `scopeManager`-based scope authority checks.
- `registerMemoryStoreTool`: local embedding + local store writes + optional `mdMirror` dual-write.
- `registerMemoryForgetTool`: local delete via local retrieval/store path.
- `registerMemoryUpdateTool`: local update + optional re-embedding path.
- `registerMemoryStatsTool` / `registerMemoryListTool`: local store-backed management read paths.
- `registerAllMemoryTools`: legacy local tool registration aggregator used by `index.ts` local branch.

Secondary local-support module deletions (no longer needed after local authority removal):
- `src/scopes.ts`
- `src/access-tracker.ts`

### 5) Local storage/retrieval modules to delete

Primary deletions:
- `src/store.ts`
- `src/retriever.ts`
- `src/embedder.ts`

Transitive cleanup required before/with deletions:
- remove type-only coupling from permanent modules that currently import local types:
  - `src/context/auto-recall-orchestrator.ts` imports `RetrievalResult` from `../retriever.js`.
  - `src/auto-recall-final-selection.ts` imports `RetrievalResult` from `./retriever.js`.
  - `src/context/reflection-prompt-planner.ts` imports `MemoryEntry` from `../store.js` and keeps local `storeList` branch.
  - `src/reflection-recall.ts` imports `MemoryEntry` from `./store.js`.
  - `src/reflection-store.ts` imports `MemoryEntry` from `./store.js` and exports local-store persistence helpers.
  - `src/benchmark.ts` imports `MemoryRetriever` types from `./retriever.js`.

## Temporary Migration-Only Schema Surfaces To Remove

In `openclaw.plugin.json`, remove after runtime cutover:
- `configSchema.properties.embedding`
- `configSchema.properties.dbPath`
- `configSchema.properties.retrieval`
- `configSchema.properties.scopes`
- `configSchema.properties.mdMirror`
- `configSchema.properties.memoryReflection.properties.storeToLanceDB`
- corresponding `uiHints` entries:
  - `embedding.*`
  - `dbPath`
  - `mdMirror.*`
  - `scopes.*`
  - `memoryReflection.storeToLanceDB`

## Permanent Local Surfaces To Keep

These stay as local adapter/context-engine responsibilities:
- `index.ts`:
  - OpenClaw hook registration,
  - remote runtime context resolution handoff,
  - remote tool registration wiring,
  - prompt injection hook plumbing.
- `src/backend-client/*`:
  - transport + actor header + retry/idempotency client logic.
- `src/backend-tools.ts`:
  - remote-backed tool surfaces (`memory_recall/store/forget/update/list/stats`) and error translation.
- `src/context/*`:
  - prompt-time planners/renderers/session-state (`auto-recall-orchestrator`, `reflection-prompt-planner`, `prompt-block-renderer`, `session-exposure-state`, `reflection-error-signals`).
- self-improvement local governance surfaces:
  - `src/self-improvement-tools.ts`, `src/self-improvement-files.ts`.

## Test Rewrite/Delete Plan (Exact Order)

### Stage T1: Remote-only config contract first

1. Rewrite `test/remote-backend-shell-integration.test.mjs`:
- replace legacy parse-time local-mode assertion (`embedding required when remote disabled`) with hard remote-only rejection assertions.
- keep all remote principal/transport/fail-open/fail-closed tests.

2. Rewrite `test/config-session-strategy-migration.test.mjs`:
- remove local embedding-centric assumptions from `baseConfig()`.
- retain only session strategy and remote-safe compatibility behavior that still exists.
- remove `autoRecallSelectionMode: "legacy"` compatibility assertion once alias is removed.

### Stage T2: Remove local CLI + migrator tests with code

3. Delete `test/cli-smoke.mjs` when `cli.ts` is deleted.
4. Delete `test/migrate-legacy-schema.test.mjs` when `src/migrate.ts` is deleted.

### Stage T3: Rewrite mixed local/remote reflection tests

5. Rewrite `test/memory-reflection.test.mjs` suites that monkeypatch local classes:
- remove `MemoryStore.prototype.*` and `MemoryRetriever.prototype.*` patching flows.
- keep/expand planner-level single-authority tests and prompt block rendering tests.
- keep context-engine tests by stubbing planner dependencies with remote-shaped DTOs.

### Stage T4: Remove local retrieval/embedder/access-tracker test suites

6. Delete local-only suites with module deletions:
- `test/retriever-trace.test.mjs`
- `test/vector-search-cosine.test.mjs`
- `test/embedder-error-hints.test.mjs`
- `test/ollama-no-apikey.test.mjs`
- `test/vllm-provider.test.mjs` sections that depend on local embedder/retriever internals.
- `test/access-tracker.test.mjs` when `src/access-tracker.ts` is deleted.

7. Remove/archive local benchmark harness when retrieval module is removed:
- `test/benchmark-runner.mjs`
- `src/benchmark.ts`

## Staged Removal Execution (Code Work)

### R0: Preflight Baseline (must pass before deletions)

Commands:
```bash
node --test test/remote-backend-shell-integration.test.mjs test/memory-reflection.test.mjs test/config-session-strategy-migration.test.mjs
npm test
rg -n "from \"\./src/(store|embedder|retriever|migrate|tools|scopes|access-tracker)\.js\"|from \"\./cli\.js\"|registerAllMemoryTools|createMemoryCLI|createMigrator|createScopeManager|AccessTracker" index.ts
rg -n "src/store\.ts|src/retriever\.ts|src/embedder\.ts|src/migrate\.ts|cli\.ts" test
```

Gate:
- baseline green before modifying deletion scope.

### R1: Hard Remote-Only Config Enforcement

Required changes:
- `parsePluginConfig` rejects config without active `remoteBackend` block.
- remove local embedding parse branch and local-mode compatibility error path.
- remove local-mode references from schema/help text.

Commands:
```bash
node --test test/remote-backend-shell-integration.test.mjs test/config-session-strategy-migration.test.mjs
rg -n "embedding config is required when remoteBackend is disabled|remoteBackend\.enabled\s*=\s*false|autoRecallSelectionMode.*legacy" index.ts test/remote-backend-shell-integration.test.mjs test/config-session-strategy-migration.test.mjs
```

Gate:
- no runtime parse path supports local authority.

### R2: Runtime Branch Cutover in `index.ts`

Required changes:
- remove `localRuntime` object and all `if (!memoryBackendClient)` branches.
- remove local tool registration and local CLI registration.
- remove local auto-capture, local reflection execution, local startup checks/backups.
- keep remote adapter/context-engine hook logic only.

Commands:
```bash
rg -n "localRuntime|registerAllMemoryTools|createMemoryCLI|createScopeManager|createMigrator|AccessTracker|mdMirror|storeToLanceDB" index.ts
node --test test/remote-backend-shell-integration.test.mjs test/memory-reflection.test.mjs
```

Gate:
- `index.ts` no longer references local-authority runtime branches.

### R3: Delete Local Authority Module Set + Resolve Type Coupling

Required changes:
- delete files: `src/store.ts`, `src/retriever.ts`, `src/embedder.ts`, `src/tools.ts`, `src/migrate.ts`, `cli.ts`, `src/scopes.ts`, `src/access-tracker.ts`.
- update/import-split permanent modules so no file imports deleted local types.
- remove local benchmark harness (`src/benchmark.ts`, `test/benchmark-runner.mjs`) if still coupled.

Commands:
```bash
for f in src/store.ts src/retriever.ts src/embedder.ts src/tools.ts src/migrate.ts cli.ts src/scopes.ts src/access-tracker.ts; do [ ! -e "$f" ] || echo "still present: $f"; done
rg -n "store\.js|retriever\.js|embedder\.js|migrate\.js|tools\.js|scopes\.js|access-tracker\.js|cli\.js" index.ts src test
```

Gate:
- no source/test import remains to deleted local modules.

### R4: Final Test/Docs/Schema Convergence

Required changes:
- remove remaining local-authority test files listed above.
- finalize `openclaw.plugin.json` remote-only schema.
- update README/docs references to remove migration-mode operational instructions.

Commands:
```bash
node --test test/remote-backend-shell-integration.test.mjs test/memory-reflection.test.mjs test/backend-client-retry-idempotency.test.mjs
npm test
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/remote-authority-reset
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/remote-authority-reset README.md
git diff --check
```

Gate:
- repo has one runtime authority model in code, schema, tests, and docs.

## Risk Notes

1. Type-coupling risk:
- context/reflection modules import types from local modules (`store.ts`/`retriever.ts`); deleting files without extracting shared types will break build/tests.

2. Behavior drift risk in reflection hooks:
- local inline reflection path and remote async enqueue path currently coexist; removing local branch can accidentally remove prompt injection behavior unless planner tests are rewritten first.

3. Upgrade break risk for existing installs:
- installs relying on `remoteBackend.enabled=false` will hard-fail after R1.
- release notes and migration messaging must be explicit before merging R1+.

4. Test-suite blind spot risk:
- deleting local suites without replacing remote/context coverage can hide regressions in prompt orchestration and principal-contract behavior.

## Rollback Strategy For Deletion Project

Rollback unit is per stage commit (`R1`, `R2`, `R3`, `R4`), not one giant change.

Operational rollback rules:
- if a stage gate fails, revert only the current stage commit and re-run previous stage gates.
- keep a pre-deletion reference tag/branch before `R1` to allow emergency restore.
- do not partial-revert shared type extraction; revert the entire stage when import graph breaks.

Emergency recovery path:
1. `git revert <stage-commit>` for failing stage.
2. run:
```bash
node --test test/remote-backend-shell-integration.test.mjs test/memory-reflection.test.mjs test/config-session-strategy-migration.test.mjs
npm test
git diff --check
```
3. reopen the stage with narrowed diff and reattempt.

## Out Of Scope For This Planning Batch

- No broad code deletion is executed in this batch.
- This document only makes the next deletion implementation batch directly executable and auditable.
