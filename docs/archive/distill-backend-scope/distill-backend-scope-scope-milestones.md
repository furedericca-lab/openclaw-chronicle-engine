---
description: Scope boundaries and milestones for distill-backend-scope.
---

# distill-backend-scope Scope and Milestones

## In Scope

- classify the current distiller residue under `scripts/` and `examples/`;
- identify which transcript-distill techniques should migrate into a future backend-native capability;
- define the capability boundary between reflection, auto-capture, and distill;
- freeze a cleanup/disposition plan for current distill-related technical debt;
- update `remote-memory-backend-2026-03-17` docs so the missing distill viewpoint is explicit.

## Out of Scope

- removing the current sidecar example code;
- changing current reflection or auto-capture behavior;
- rewriting README guidance beyond what is needed for architectural clarity.
- shipping `session-transcript` source resolution or provider-driven extraction/reduce behavior.

## Milestones

### Milestone 1 — Distill capability decomposition

Acceptance gate:

- current distiller residue is classified file-by-file;
- reusable techniques are separated from sidecar-only deployment details;
- the current state of reflection, auto-capture, and distill is documented without ambiguity;
- each residue item has an initial cleanup class.

### Milestone 2 — Backend-native target architecture

Acceptance gate:

- technical docs and contracts describe a future backend-native distill job family;
- the plan clearly states what should be absorbed, rejected, left as example-only, or deleted later as debt cleanup.

### Milestone 3 — Remote backend documentation alignment

Acceptance gate:

- `docs/remote-memory-backend-2026-03-17` explicitly covers the distill viewpoint;
- the remote backend docs no longer leave `jsonl_distill.py` and the distiller example hanging outside the architecture story.
- cleanup/disposition intent is documented so old residue does not read like indefinite supported architecture.

## Dependencies

- Milestone 1 blocks architecture and contract work.
- Milestone 2 depends on Milestone 1 classification.
- Milestone 3 depends on Milestone 2 so the remote backend docs reflect the same final position.

## Exit Criteria

- the repo has one active scope describing future backend-native distill alignment;
- the repo has one active scope describing how current distill residue will be cleaned up over time;
- the remote backend snapshot explicitly explains that `inline-messages` distill execution is shipped while `session-transcript` source resolution remains deferred, and how it relates to reflection and auto-capture;
- future implementation work can begin without re-discovering what `jsonl_distill.py` is for or whether it is canonical.
