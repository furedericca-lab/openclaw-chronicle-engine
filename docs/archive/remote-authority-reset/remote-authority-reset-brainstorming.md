---
description: Brainstorming and decision framing for remote-authority-reset.
---

# remote-authority-reset Brainstorming

## Problem

The repository currently describes the architecture through two active documentation tracks: `context-engine-split` and `remote-memory-backend`. That wording makes the system look like two competing architecture directions, even though the desired end state is singular: a Rust remote backend as the only memory authority, a thin OpenClaw adapter, and a local context-engine that handles prompt-time context management only.

Success means the repo has one canonical architecture story, one cleanup plan, one module-boundary map, and one phased refactor plan. Historical reasoning must remain preserved, but only in archive form.

## Scope

In scope:
- archive the current active docs under `docs/context-engine-split/` and `docs/remote-memory-backend/`;
- define the target three-layer architecture;
- define cleanup and refactor milestones for docs, TypeScript shell code, local context-engine code, and Rust backend boundaries;
- update top-level references so canonical docs point to the new scope;
- mark local-authority code paths as legacy migration-only paths pending removal.

Out of scope:
- implementing the full refactor in this doc batch;
- changing published runtime APIs immediately;
- deleting historical archive material;
- inventing a second shipped plugin kind;
- adding new product scope unrelated to remote authority or context orchestration.

## Constraints

- Rust remote backend must be the only authority for storage, retrieval, ranking, ACL, scope, and reflection persistence/execution.
- The local OpenClaw integration layer must stay thin and must not reintroduce local backend authority.
- The local context-engine may control recall timing, prompt block rendering, session-local suppression, and error-signal dedupe, but it must consume backend-authoritative rows only.
- No long-term dual-authority mode is allowed.
- Historical documents must remain accessible after archival.
- The plan must be implementable in the existing repo layout: `backend/`, `src/`, `test/`, `README.md`, `README_CN.md`, `openclaw.plugin.json`.

## Options

### Option A — Keep both active doc tracks and add a summary note
- Complexity: low.
- Migration impact: low.
- Reliability: poor; naming drift remains.
- Operational burden: medium; future contributors still need to reconcile two narratives.
- Removal-readiness: poor.

### Option B — Merge both tracks into a new canonical architecture scope and archive the old tracks
- Complexity: medium.
- Migration impact: medium.
- Reliability: strong; one authoritative narrative.
- Operational burden: low; future work follows one plan.
- Removal-readiness: strong, because history remains archived.

### Option C — Keep only `remote-memory-backend` active and fold context-engine into implementation details without its own explicit role
- Complexity: low-medium.
- Migration impact: medium.
- Reliability: mixed; remote authority is clear, but local orchestration responsibilities become too implicit.
- Operational burden: medium; people will keep rediscovering where prompt-time logic belongs.
- Removal-readiness: mixed.

## Decision

Choose **Option B**.

Rationale:
- It matches the intended end state without pretending the previous two active scopes are still first-class directions.
- It preserves the value of the previous work as archive material instead of erasing it.
- It keeps `context-engine` explicit as a local orchestration layer, which is important for OpenClaw integration and for preventing backend scope creep.
- It gives the repo a single migration and cleanup plan that can drive implementation directly.

Alternatives rejected:
- Option A leaves the core confusion intact.
- Option C under-specifies the local orchestration layer and invites future architectural drift.

## Risks

- Top-level docs may still contain old references after the archive move.
- Existing closeout notes may still talk in the old two-track language.
- Implementation may still contain transitional naming that does not fit the new canonical story.
- Some tests may still encode local-vs-remote transitional assumptions instead of the target architecture.

## Open Questions

- Which local-authority migration paths can be deleted immediately versus after one additional compatibility release?
- Should the Rust backend remain LanceDB-specific internally, or should the docs describe it as a generic memory/RAG backend with LanceDB as the current implementation detail?
- How aggressively should `memory-lancedb-pro` naming be cleaned up in user-facing docs versus package identity preservation?
