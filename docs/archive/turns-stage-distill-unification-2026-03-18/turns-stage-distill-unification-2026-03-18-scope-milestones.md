---
description: Scope boundaries and milestones for turns-stage-distill-unification-2026-03-18.
---

# turns-stage-distill-unification-2026-03-18 Scope and Milestones

## In Scope

- Remove command-triggered reflection generation from plugin/runtime flow.
- Remove plugin-side `/new` / `/reset` reflection source/job orchestration.
- Reframe distill as the sole trajectory-to-new-knowledge write path.
- Evolve distill extraction semantics from the rejected token-chunk map-stage idea into **turns-stage lesson extraction**.
- Merge reflection conflict areas into distill ownership:
  - `session-lessons`: lesson/cause/fix/prevention/stable decision/durable practice
  - `governance-candidates`: worth-promoting learnings / skill extraction candidates / AGENTS/SOUL/TOOLS promotion candidates
- Downgrade `Derived` / `Open loops / next actions` style outputs into distill-owned artifact subtypes (`follow-up-focus` / `next-turn-guidance`).
- Clean up TS runtime code, tests, and docs that exist only for the removed command-triggered reflection generation path.
- Preserve cadence-driven automatic distill (`distill.everyTurns`) as the primary trigger surface.

## Out of Scope

- Reintroducing the old `jsonl_distill.py` sidecar architecture.
- Keeping backward-compatibility shims for `/new` / `/reset` reflection generation.
- Large redesign of unrelated generic recall or memory mutation APIs.
- Reworking self-improvement local governance tools beyond necessary wording or test cleanup.
- Mandatory database migration for historical rows unless implementation proves schema corruption risk.

## Milestones

### Milestone 1 — Semantic contract reset
- Freeze the new terminology:
  - distill owns all trajectory-derived writes
  - reflection no longer owns command-triggered generation
  - turns-stage lesson extraction replaces map-stage wording
- Identify exact public surfaces to remove or retain.

### Milestone 2 — TS/plugin cleanup
- Remove `/new` / `/reset` reflection enqueue hooks from `index.ts`.
- Remove plugin-side use of reflection source/job client paths where no longer needed.
- Remove reflection-job status management tool if it becomes dead.
- Rewrite or delete affected TS tests.

### Milestone 3 — Backend distill absorption
- Extend backend distill semantics/tests so `session-lessons` (or a tightly scoped successor mode) can cover the retained reflection-like extraction value.
- Preserve cadence-driven `session-transcript` flow.
- Keep deterministic/evidence-based output guarantees.

### Milestone 4 — Docs and release cleanup
- Update README / README_CN / architecture docs.
- Remove stale mention of command-triggered reflection generation.
- Document turns-stage lesson extraction and the new ownership model.

## Dependencies

- `index.ts`
- `src/backend-tools.ts`
- `src/backend-client/client.ts`
- `src/backend-client/types.ts`
- `src/context/reflection-prompt-planner.ts`
- `test/remote-backend-shell-integration.test.mjs`
- `test/memory-reflection.test.mjs`
- `backend/tests/phase2_contract_semantics.rs`
- `README.md`
- `README_CN.md`

## Exit Criteria

- No plugin runtime path writes new knowledge because `/new` or `/reset` was invoked.
- `distill.everyTurns` remains the canonical cadence trigger for automatic knowledge generation.
- Distill documentation explicitly describes turns-stage lesson extraction and no longer claims map-stage/token-chunk extraction is the intended future path.
- Removed reflection-generation APIs/hooks/tests/docs leave no stale user-facing promises.
- The resulting architecture can be described in one sentence without ambiguity: **distill generates and writes knowledge; reflection only recalls/injects existing knowledge if retained at all.**
