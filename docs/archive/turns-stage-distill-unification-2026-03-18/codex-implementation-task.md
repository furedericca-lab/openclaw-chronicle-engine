# Codex Implementation Task — turns-stage-distill-unification-2026-03-18

You are implementing the approved scope in this verify worktree.

## Source of truth
Read and follow these docs first:
- `docs/turns-stage-distill-unification-2026-03-18/turns-stage-distill-unification-2026-03-18-contracts.md`
- `docs/turns-stage-distill-unification-2026-03-18/turns-stage-distill-unification-2026-03-18-scope-milestones.md`
- `docs/turns-stage-distill-unification-2026-03-18/turns-stage-distill-unification-2026-03-18-implementation-research-notes.md`
- `docs/turns-stage-distill-unification-2026-03-18/turns-stage-distill-unification-2026-03-18-technical-documentation.md`
- `docs/turns-stage-distill-unification-2026-03-18/task-plans/4phases-checklist.md`
- `docs/turns-stage-distill-unification-2026-03-18/task-plans/phase-2-turns-stage-distill-unification-2026-03-18.md`
- `docs/turns-stage-distill-unification-2026-03-18/task-plans/phase-3-turns-stage-distill-unification-2026-03-18.md`
- `docs/turns-stage-distill-unification-2026-03-18/task-plans/phase-4-turns-stage-distill-unification-2026-03-18.md`

## Frozen requirements

### 1. Cleanup posture
- Do **not** implement compatibility shims.
- Do **not** preserve rollback compatibility for removed reflection-generation behavior.
- Delete stale code/tests/docs directly when they only exist for the removed path.

### 2. Ownership split
- `distill` is the only write path that derives new knowledge from session trajectories.
- `reflection` remains **recall / injection only**.
- No independent reflection generation mode remains.

### 3. Distill mode semantics
#### `session-lessons` must own:
- lesson
- cause
- fix
- prevention
- stable decision / durable practice

#### `governance-candidates` must own:
- worth-promoting learnings
- skill extraction candidates
- AGENTS/SOUL/TOOLS promotion candidates

#### `Derived` / `Open loops / next actions`
- must be downgraded into distill-owned artifact subtypes
- use names:
  - `follow-up-focus`
  - `next-turn-guidance`
- do not preserve a separate reflection/invariant persistence pipeline for them

### 4. Trigger model
- remove `/new` / `/reset` reflection generation flow
- keep cadence-driven generation via `agent_end` + `distill.everyTurns`

## Required implementation direction

### TypeScript/runtime cleanup
Remove command-triggered reflection generation surfaces from runtime/plugin code, including the plugin-side calls and tests tied to:
- `POST /v1/reflection/source`
- `POST /v1/reflection/jobs`
- `command:new` reflection hook registration
- `command:reset` reflection hook registration
- reflection-job status tooling if dead after cleanup

Reflection recall/injection code may remain only if it is read-only and still useful.

### Backend distill absorption
Implement the retained useful behavior under backend-native distill.
Prefer evolving `session-lessons` rather than introducing a brand new mode unless a hard schema split is unavoidable.

### Documentation cleanup
Update `README.md`, `README_CN.md`, and any affected architecture docs to match the new ownership model and remove stale reflection-generation promises.

## Verification requirements
Run and report:
- `npm test -- --test-name-pattern="reflection|distill|session-strategy"`
- `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/turns-stage-distill-unification-2026-03-18`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/turns-stage-distill-unification-2026-03-18 README.md`

Also run targeted grep checks to prove removal of command-triggered reflection-generation references.

## Delivery contract
At the end, provide:
- status
- verification layer reached (`test` / `check` / `confirm`)
- changed files summary
- any blockers or follow-up risks
- concise explanation of how the final code matches the frozen ownership split
