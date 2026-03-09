---
description: Planning and seam-mapping tasks for context-engine-split.
---

# Tasks: context-engine-split

## Input
- `README.md`
- `openclaw.plugin.json`
- `index.ts`
- `src/recall-engine.ts`
- `src/auto-recall-final-selection.ts`
- `src/reflection-recall.ts`
- `src/reflection-store.ts`
- `test/memory-reflection.test.mjs`
- `docs/context-engine-split/*.md`

## Canonical architecture / Key constraints
- The repository must remain a `memory` plugin in this branch.
- Backend retrieval/storage modules remain authoritative for persistence, scoring, scopes, and tool APIs.
- Hook-driven behavior (`before_agent_start`, `before_prompt_build`, `after_tool_call`, `agent_end`, `command:new`, `command:reset`) must be preserved.
- Phase 1 is docs/seam mapping only; no contract flip.

## Format
- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 1: Contract and seam map
Goal: establish the concrete internal refactor contract, seam map, and verification matrix before code movement.
Definition of Done: scope docs are repo-specific, active paths are enumerated, and implementation can start without redoing discovery.

Tasks:
- [ ] T001 [Docs] Normalize the scoped planning docs with repo-specific findings and boundaries.
  - DoD: `docs/context-engine-split/*.md` contain concrete file paths, actual hook names, and explicit in-scope/out-of-scope statements for this repo.
- [ ] T002 [P] [QA] Build the active-path verification matrix for migration safety.
  - DoD: phase docs and checklist enumerate verification coverage for generic auto-recall, reflection recall, config compatibility, `/new`/`/reset`, and self-improvement paths with concrete commands.
- [ ] T003 [Security] Define refactor guardrails for scope filtering, untrusted-data wrapping, and no-contract-flip behavior.
  - DoD: technical docs/contracts capture the security-sensitive seams and compatibility rules that implementation must preserve.

Checkpoint: phase-1 docs are complete, concrete, and referenced by the implementation phases before any code movement starts.

## Dependencies & Execution Order
- Phase 1 blocks all others.
- Tasks marked `[P]` may run concurrently only when they do not touch the same files.
