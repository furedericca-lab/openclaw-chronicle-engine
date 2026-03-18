---
description: Implementation research notes for turns-stage-distill-unification-2026-03-18.
---

# turns-stage-distill-unification-2026-03-18 Implementation Research Notes

## Current-State Findings

### Distill already has the correct trigger shape
Evidence from current code/docs/tests:
- `index.ts` appends transcript rows on `agent_end` and conditionally enqueues automatic distill when `distill.everyTurns` is crossed.
- `test/remote-backend-shell-integration.test.mjs` verifies assistant-only batches do not count and the fifth user-turn boundary triggers one enqueue.
- `README.md` / `README_CN.md` already position automatic distill as a backend-native cadence-driven path.

Implication:
- the target trigger mechanism already exists and should be promoted, not replaced.

### Reflection generation is command-coupled and semantically redundant
Evidence from current code/docs/tests:
- `index.ts` registers `command:new` / `command:reset` hooks under the memory-reflection block.
- the same block loads `reflection/source` and enqueues `reflection/jobs`.
- integration tests verify non-blocking reflection enqueue on `/new` / `/reset`.
- README sections still document `/new` / `/reset` reflection flow as an active supported path.

Implication:
- this path is a clean removal candidate because it is localized, test-covered, and separable from cadence-driven distill.

### The rejected piece is the old map-stage framing, not lesson extraction itself
Current docs say distill intentionally does not do:
- language-adaptive extraction
- model-backed map-stage lesson extraction

Implication:
- the architecture can adopt lesson extraction under a new turns-stage semantic contract without contradicting the rejection of token-chunk map-stage behavior.

## Proposed Semantic Reset

### Before
- reflection: both generation and recall/injection
- distill: deterministic backend reduction and optional memory persistence
- overlap: both produce trajectory-derived learnings in different trigger planes

### After
- distill: all generation/extraction/write behavior from trajectories
- `session-lessons`: lesson/cause/fix/prevention/stable decision/durable practice
- `governance-candidates`: worth-promoting learnings / skill extraction candidates / AGENTS/SOUL/TOOLS promotion candidates
- `Derived` / `Open loops / next actions`: downgraded into distill-owned artifact subtypes `follow-up-focus` / `next-turn-guidance`
- reflection: recall/injection only
- self-improvement: local governance/logging only

## Design Options Considered

### Option A — Keep reflection jobs, just retime them to cadence
Rejected.
Reason:
- preserves duplicate write semantics under two names
- keeps user-facing ambiguity around “reflection vs distill”
- does not satisfy the desired semantic cleanup

### Option B — Move reflection-like extraction into distill but keep a separate distill mode immediately
Possible, but not preferred for the first pass.
Reason:
- may be useful later if output schema truly diverges
- adds migration surface too early

### Option C — Absorb reflection-generation semantics into `session-lessons` first
Preferred starting plan.
Reason:
- minimal surface churn
- keeps existing cadence trigger intact
- allows backend tests to evolve from current lesson artifacts toward the richer retained extraction behavior

## Technical Risk Notes

1. `index.ts` currently entangles reflection generation and reflection recall in the same broad `sessionStrategy === "memoryReflection"` block.
   - cleanup should separate recall-only code from removed command-hook code to avoid accidental collateral deletion.

2. `src/backend-tools.ts` currently exposes `memory_reflection_status`.
   - if reflection jobs disappear entirely, the management surface should be removed in the same pass.

3. `README*` currently promises reflection source loading and reflection jobs.
   - docs must be updated atomically with code removal to avoid false contracts.

4. Backend tests currently encode reflection endpoints as part of frozen semantics.
   - those tests must be intentionally rewritten, not left to fail ambiguously.

## Open Technical Questions

1. Should reflection-category rows continue to exist as a memory category after generation is absorbed into distill, or should distill emit only ordinary durable categories plus artifacts?
2. Is `session-lessons` sufficient as the final mode name, or should a later rename such as `session-turns-lessons` be considered after behavior stabilizes?
3. Should prompt-time reflection recall keep reading `reflection` rows, or should it eventually read distill-produced rows through a more explicit recall channel?

## Recommended Implementation Order

1. Contract/docs freeze.
2. Remove plugin command-hook generation path.
3. Remove/reshape reflection management tool surfaces.
4. Extend backend distill tests and extraction semantics.
5. Update recall-side wording/config so reflection means recall/injection only.
6. Final docs + residual scan.
