---
description: Phase 4 release-closeout notes for remote-authority-reset.
---

# remote-authority-reset Phase 4 Release Closeout Notes

## Scope of Change

### Phase 2 (hard remote-only enforcement)
- runtime parsing now requires `remoteBackend.enabled=true`;
- local-authority parse/runtime branches were removed from `index.ts`;
- local-authority schema/help fields were removed from `openclaw.plugin.json`.

### Phase 3 (hard deletion)
- deleted remaining local-authority modules/tests (`src/store.ts`, `src/retriever.ts`, `src/embedder.ts`, `src/scopes.ts`, `src/access-tracker.ts`, and related tests/benchmark harness);
- removed transitive type coupling from permanent modules via `src/memory-record-types.ts`;
- converged retained tests to remote-only + context-engine seams.

### Phase 4 (closeout)
- completed full retained regression + doc hygiene batch;
- removed residual wording drift from active README/schema/help/canonical docs;
- re-confirmed remote principal and backend-owned scope invariants;
- recorded release-cut and rollback discipline for operation teams.

## User-Visible Upgrade Breakages

- configs depending on `remoteBackend.enabled=false` no longer start;
- local-authority runtime fields (`embedding`, `dbPath`, `retrieval`, `scopes`, `mdMirror`, `memoryReflection.storeToLanceDB`) are not supported runtime config;
- local `memory-pro` CLI and local migration command surfaces are removed.

## Rollout Cautions

- deploy only with a reachable remote backend and valid `remoteBackend.authToken`;
- ensure runtime context provides both `userId` and `agentId` (recall degrades, writes/management/enqueue block without principal);
- treat backend ACL/scope as authoritative and do not design callers around client scope override.

## Remote Principal / Scope Invariant Re-Confirmation

- principal enforcement: `src/backend-client/runtime-context.ts` + `src/backend-tools.ts` still require runtime principal identity for write/update/delete/list/stats/enqueue paths;
- backend-owned scope: remote tools expose no client `scope` argument and use backend visibility decisions;
- `/new` and `/reset` reflection continue to enqueue remote jobs (non-blocking), not local persistence.

## Rollback Discipline

- rollback unit is a pre-deletion release/worktree/tag, not in-place patching;
- do not attempt to re-enable deleted local runtime paths in the same tree;
- if rollback is required, cut over to the last known-good pre-deletion artifact and then re-run post-upgrade verification before re-promotion.

## Post-Upgrade Verification Commands

```bash
npm test
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/remote-authority-reset
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/remote-authority-reset README.md
find docs/archive/2026-03-15-architecture-reset -maxdepth 2 -type f | sort
find docs/remote-authority-reset -maxdepth 2 -type f | sort
git diff --check
```

## Closeout Status

Phase 4 closeout is complete when the command batch above passes and corresponding evidence is recorded in `docs/remote-authority-reset/task-plans/4phases-checklist.md`.
