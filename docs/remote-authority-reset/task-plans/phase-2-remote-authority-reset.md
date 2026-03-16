---
description: Phase 2 execution plan for hard remote-only runtime enforcement before legacy module deletion.
---

# Tasks: remote-authority-reset (Phase 2)

## Input
- `docs/remote-authority-reset/remote-only-local-authority-removal-plan.md`
- `index.ts`
- `openclaw.plugin.json`
- `README.md`
- `test/remote-backend-shell-integration.test.mjs`
- `test/config-session-strategy-migration.test.mjs`

## Canonical architecture / Key constraints
- Remote backend authority is the only supported runtime authority.
- No runtime/config path may continue to support `remoteBackend.enabled=false`.
- This phase may remove runtime branches and schema fields, but should not do broad transitive file deletion yet.
- Single-authority planner guards and self-improvement extraction work must remain intact.

## Format
- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 2 Goal
Harden runtime/config contract to remote-only and isolate remaining local-authority code as deletable dead paths.

Definition of Done:
- Parse/runtime contract no longer accepts local-authority mode.
- Schema/help text no longer exposes local-authority runtime fields as operational config.
- Remote contract tests are updated and passing.

## Tasks
- [x] T021 [Agentic] Enforce remote-only parse contract in `index.ts`.
  - DoD: `parsePluginConfig` no longer contains local embedding parse branch or local-mode compatibility errors.

- [x] T022 [Agentic] Remove local runtime branches from `index.ts` that are blocked only by parse compatibility.
  - DoD: local tool/CLI registration and local startup/shutdown runtime branches are deleted or made unreachable by design.

- [x] T023 [Config] Prune migration-only schema/help fields from `openclaw.plugin.json`.
  - DoD: local-authority config surfaces (`embedding`, `dbPath`, `retrieval`, `scopes`, `mdMirror`, `memoryReflection.storeToLanceDB`) are removed from active schema/uiHints.

- [x] T024 [QA] Rewrite config-contract tests for remote-only enforcement.
  - DoD: `test/remote-backend-shell-integration.test.mjs` and `test/config-session-strategy-migration.test.mjs` assert hard remote-only behavior and pass.

- [x] T025 [Security] Re-verify remote principal and no-client-scope contract remains intact.
  - DoD: remote tool tests still prove no `scope`/`scopeFilter` payload authority and missing-principal write paths fail closed.

## Phase 2 Verification
```bash
node --test test/remote-backend-shell-integration.test.mjs test/config-session-strategy-migration.test.mjs
rg -n "registerAllMemoryTools|createMemoryCLI|localRuntime|createScopeManager|createMigrator|AccessTracker" index.ts
rg -n "\"embedding\"|\"dbPath\"|\"retrieval\"|\"scopes\"|\"mdMirror\"|storeToLanceDB" openclaw.plugin.json
git diff --check
```

## Dependencies & Execution Order
- Phase 1 is complete and blocks this phase.
- `T021` must land before `T022` and `T024`.
- `T023` can run in parallel with `T022`, then must be validated with `T024`.
- `T025` runs after `T021`-`T024`.

Checkpoint:
- Runtime behavior is remote-only enforced and ready for Phase 3 physical module deletions.

## Execution Notes (2026-03-15, cleanup #2)
- `index.ts` now enforces remote-only parse/runtime behavior and no longer retains reachable local runtime branches.
- `openclaw.plugin.json` removed active local-authority schema/ui surfaces and enforces `remoteBackend.enabled: true`.
- `test/remote-backend-shell-integration.test.mjs` and `test/config-session-strategy-migration.test.mjs` were rewritten to remote-only assertions.
