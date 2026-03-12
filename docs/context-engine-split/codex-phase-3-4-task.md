Continue the already-started context-engine-split work in this worktree.

Repo:
- /root/verify/memory-lancedb-pro-context-engine-split
- branch: feat/context-engine-split

Previous verified run:
- Phase 2 code extraction already landed in this worktree and passed `npm test`.
- Do NOT redo Phase 2 implementation unless needed for Phase 3/4 completion.

Read first:
- docs/context-engine-split/context-engine-split-brainstorming.md
- docs/context-engine-split/context-engine-split-implementation-research-notes.md
- docs/context-engine-split/context-engine-split-scope-milestones.md
- docs/context-engine-split/technical-documentation.md
- docs/context-engine-split/context-engine-split-contracts.md
- docs/context-engine-split/task-plans/phase-3-context-engine-split.md
- docs/context-engine-split/task-plans/phase-4-context-engine-split.md
- docs/context-engine-split/task-plans/4phases-checklist.md

Goal:
- Complete Phase 3 and Phase 4 only.

Required work:
1. Update README.md and README_CN.md so architecture/module-boundary sections reflect the new internal split:
   - backend storage/retrieval remains in backend modules
   - prompt/context orchestration now lives under `src/context/*`
   - do NOT claim the plugin already ships as a standalone ContextEngine
2. Review hook-path parity / failure-mode wording as needed in docs.
3. Run the required doc hygiene scans:
   - bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/context-engine-split
   - bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/context-engine-split README.md README_CN.md
4. Re-run verification as needed (at minimum `npm test` if doc/code changes require it; if docs-only after verified code, use judgment but record evidence).
5. Update `docs/context-engine-split/task-plans/4phases-checklist.md` so Phase 3 and Phase 4 reflect actual completion/evidence/results.
6. Add a concise future adapter/handoff note in the docs that explains what a later thin ContextEngine adapter should consume and what remains backend-owned.

Constraints:
- Do NOT change `openclaw.plugin.json` kind.
- Do NOT rename public config keys or tool names.
- Keep claims grounded in actual file-path evidence.
- Prefer minimal, reviewable edits.

Deliverable summary expectations:
- completed vs deferred
- exact touched files
- doc scan results
- test results
- any remaining reviewer notes
