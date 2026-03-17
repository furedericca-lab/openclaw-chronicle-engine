Implement the context-engine-split refactor in this worktree.

Repo:
- /root/verify/memory-lancedb-pro-context-engine-split
- branch: feat/context-engine-split

Read first:
- docs/context-engine-split/context-engine-split-brainstorming.md
- docs/context-engine-split/context-engine-split-implementation-research-notes.md
- docs/context-engine-split/context-engine-split-scope-milestones.md
- docs/context-engine-split-2026-03-15/context-engine-split-2026-03-15-technical-documentation.md
- docs/context-engine-split/context-engine-split-contracts.md
- docs/context-engine-split/task-plans/phase-2-context-engine-split.md
- docs/context-engine-split/task-plans/phase-3-context-engine-split.md
- docs/context-engine-split/task-plans/4phases-checklist.md

Goal:
- Execute Phase 2 of the plan: extract prompt/context orchestration out of index.ts into dedicated modules while preserving the current memory-plugin contract and runtime behavior.

Hard constraints:
- Do NOT change openclaw.plugin.json kind from memory.
- Do NOT rename or break public config keys or tool names.
- Keep backend ownership of storage/retrieval/scopes/tools in existing backend modules.
- Preserve active paths: before_agent_start, before_prompt_build, after_tool_call, agent_end, command:new, command:reset.
- Prefer a thin compatibility refactor: modularize now, do not pretend a standalone ContextEngine already exists.
- Keep README claims accurate: internal split is okay; shipped ContextEngine migration is not.

Implementation targets:
- Make index.ts visibly thinner by delegating orchestration logic.
- Introduce dedicated modules for at least:
  - generic auto-recall planning/provider logic
  - reflection recall + error-signal prompt planning
  - session-local exposure state ownership
  - prompt block rendering/composition
- Preserve existing behavior and tests as much as possible.
- Update docs touched by architecture descriptions if needed.
- Update docs/context-engine-split/task-plans/4phases-checklist.md with implementation evidence/results.

Verification:
- Run at least:
  - npm test
- If needed also run focused commands from the docs.

Deliverable summary expectations:
- concise status
- changed files
- behavior preserved vs intentionally deferred
- test results
- blockers/risks if any
