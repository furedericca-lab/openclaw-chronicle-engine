# Codex Task: remote-authority-reset implementation batch

## Repo
- `/root/verify/memory-lancedb-pro-context-engine-split`
- branch: `dev/context-engine-split`

## Goal
Implement the architecture reset plan defined under `docs/remote-authority-reset/`.

Target architecture:
- Rust remote backend is the only memory/RAG authority.
- The OpenClaw TypeScript layer is a thin adapter only.
- The local context-engine owns prompt-time orchestration only.

## Required source documents
Read and follow these first:
- `docs/remote-authority-reset/README.md`
- `docs/remote-authority-reset/remote-authority-reset-brainstorming.md`
- `docs/remote-authority-reset/remote-authority-reset-implementation-research-notes.md`
- `docs/remote-authority-reset/remote-authority-reset-scope-milestones.md`
- `docs/archive/remote-authority-reset/remote-authority-reset-technical-documentation.md`
- `docs/remote-authority-reset/remote-authority-reset-contracts.md`
- `docs/remote-authority-reset/task-plans/4phases-checklist.md`
- `docs/remote-authority-reset/task-plans/phase-1-remote-authority-reset.md`
- `docs/remote-authority-reset/task-plans/phase-2-remote-authority-reset.md`
- `docs/remote-authority-reset/task-plans/phase-3-remote-authority-reset.md`
- `docs/remote-authority-reset/task-plans/phase-4-remote-authority-reset.md`

## What to do
Complete as much of Phases 2-4 as is safely possible in one coherent implementation batch.

Priority order:
1. Audit and clean TypeScript adapter boundaries in:
   - `index.ts`
   - `src/backend-client/*`
   - `src/backend-tools.ts`
   - `src/tools.ts`
2. Audit and clean local context-engine boundaries in:
   - `src/context/*`
3. Remove or isolate transitional local-authority logic that conflicts with the canonical architecture.
4. Update tests to match the canonical boundary model.
5. Clean user-facing wording in:
   - `README.md`
   - `README_CN.md`
   - `openclaw.plugin.json`
   - relevant comments/log strings
6. Update `docs/remote-authority-reset/task-plans/4phases-checklist.md` with real execution evidence/results.

## Constraints
- Do not touch global Codex config.
- Do not use the canonical workspace repo path; work only in this verify worktree.
- Do not delete archive history.
- Keep remote authority as the only supported runtime authority; any local-authority path must stay deprecated, migration-only, and pending removal.
- Do not introduce a new plugin kind.
- Do not move prompt rendering / prompt-time session-local orchestration into the Rust backend.
- Do not modify `backend/target/`.
- Preserve package identity `memory-lancedb-pro`.

## Expected implementation stance
- Prefer small, reviewable patches over broad speculative rewrites.
- If a cleanup is too risky, document it in the checklist instead of forcing it.
- Keep docs and code consistent.
- If tests reveal a contract mismatch, align implementation and docs to the canonical architecture where reasonable.

## Verification
Run meaningful verification for touched areas.
Minimum target set:
- `node --test test/remote-backend-shell-integration.test.mjs`
- `node --test test/memory-reflection.test.mjs`
- `node --test test/config-session-strategy-migration.test.mjs`
- `npm test`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/remote-authority-reset`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/remote-authority-reset README.md`
- `git diff --check`

## Deliverable
When done, leave the repo in a coherent state and summarize:
- status: done | partial | blocked | failed
- changed files
- key architectural cleanup completed
- tests/verification run with results
- any deferred items with reasons
