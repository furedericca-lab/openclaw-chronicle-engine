---
description: Scope boundaries and milestones for adapter-surface-closeout-2026-03-17.
---

# adapter-surface-closeout-2026-03-17 Scope and Milestones

## In Scope

- add the missing adapter/plugin surface for backend-native distill enqueue/status;
- add the missing adapter/plugin surface for backend recall debug trace routes;
- define and implement the compatibility story for stale `memoryReflection` local-generation config and helper code;
- align `setwise-v2` runtime behavior with the stable ordinary recall DTO contract;
- remove, relocate, or explicitly demote residual TS/package/docs artifacts proved unused or misleading in the supported runtime;
- update README, README_CN, plugin schema descriptions, and runtime docs so the shipped surface matches the code.

## Out of Scope

- changing backend ownership of persistence, recall, ranking, ACL, scope, reflection execution, or distill execution;
- widening ordinary `/v1/recall/*` DTOs to include embeddings or raw ranking internals;
- adding a new admin/control-plane authentication model;
- redesigning prompt rendering or moving `src/context/*` into the backend;
- introducing unrelated architecture changes outside this repo/runtime.

## Milestones with explicit acceptance gates

### Milestone 1 - Scope freeze and contract decisions

Acceptance gate:

- scope docs freeze the exact disposition of:
  - distill shell surface
  - recall debug trace shell surface
  - stale `memoryReflection` config/runtime helpers
  - `setwise-v2` runtime semantics
  - residual TS/package cleanup targets
- task plans map those decisions into auditable implementation phases.

### Milestone 2 - Adapter surface completion

Acceptance gate:

- plugin/backend-client code exposes the selected distill and debug recall management surface;
- tool registration and typed client methods are concrete and test-backed;
- fail-open/fail-closed rules remain explicit and unchanged where required.

### Milestone 3 - Local residual cleanup and semantic alignment

Acceptance gate:

- dead local reflection-generation runtime code is removed or clearly demoted from supported runtime behavior;
- stale config keys are either deprecated/ignored or removed with an explicit migration story;
- `setwise-v2` is aligned with the actual runtime data it receives, with tests proving the intended behavior.

### Milestone 4 - Residual debt closeout

Acceptance gate:

- README/schema/package/test naming and dependency residue match the supported runtime story;
- import/use-site scans confirm no accidental production dependency on helpers classified as non-runtime;
- checklist records the verification evidence and any residual follow-up.

## Dependencies

- Milestone 1 blocks all later milestones.
- Milestone 2 depends on Milestone 1.
- Milestone 3 depends on Milestones 1-2 because config and local cleanup depend on the chosen shell-surface contract.
- Milestone 4 depends on Milestones 1-3 because docs/package cleanup must reflect the final implemented state rather than speculative design.

## Exit Criteria per milestone

- Milestone 1 exit:
  - docs are concrete enough that implementation can proceed without rediscovery.
- Milestone 2 exit:
  - distill/debug shell surfaces are implemented and covered by Node integration tests.
- Milestone 3 exit:
  - no supported runtime path depends on dead local reflection-generation code or on unshipped setwise assumptions.
- Milestone 4 exit:
  - docs, schema, package metadata, and residual helper placement no longer contradict the remote-authority runtime.
