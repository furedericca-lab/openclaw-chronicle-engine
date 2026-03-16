---
description: Scope boundaries and milestones for remote-authority-reset.
---

# remote-authority-reset Scope and Milestones

## In Scope

- preserve archive history while keeping one active canonical architecture;
- enforce remote backend as the only supported runtime authority;
- delete legacy local-authority runtime branches and modules;
- keep adapter + context-engine local responsibilities explicit and narrow;
- converge tests/schema/docs to the post-deletion remote-only runtime model.

## Out of Scope

- shipping a new standalone OpenClaw plugin kind;
- moving prompt block rendering into the Rust backend;
- re-introducing long-term dual-authority runtime support;
- deleting historical archive material.

## Milestones

### Milestone 1 — Documentation reset and canonical architecture freeze (completed)
Acceptance gate:
- old active architecture docs are archived under `docs/archive/2026-03-15-architecture-reset/`;
- canonical docs exist under `docs/remote-authority-reset/`;
- the repo has one explicit architecture sentence and one cleanup/removal track.

### Milestone 2 — Hard remote-only runtime/config enforcement
Acceptance gate:
- `index.ts` parse/runtime contract no longer supports local-authority mode;
- `openclaw.plugin.json` no longer exposes local-authority runtime config surfaces as active fields;
- remote config contract tests are updated and passing.

### Milestone 3 — Local-authority implementation deletion
Acceptance gate:
- local module set (`src/store.ts`, `src/retriever.ts`, `src/embedder.ts`, `src/tools.ts`, `src/migrate.ts`, `src/scopes.ts`, `src/access-tracker.ts`, `cli.ts`) is removed;
- permanent modules no longer import deleted local types/modules;
- tests no longer depend on local-authority runtime classes.

### Milestone 4 — Verification and release closeout
Acceptance gate:
- post-deletion regression and doc hygiene pass;
- user-facing docs/config/help text fully reflect remote-only runtime support;
- release notes include stage-level rollback instructions.

## Dependencies

- Milestone 1 is complete and blocks all later work.
- Milestone 2 must complete before Milestone 3 to avoid mixed runtime contracts.
- Milestone 3 must complete before Milestone 4 to ensure final verification matches actual code.

## Exit Criteria

- one supported runtime authority only: remote backend authority;
- adapter/context-engine split remains explicit and stable;
- no executable local-authority runtime path exists in code, config schema, or tests;
- phase checklist contains evidence-backed verification for completed milestones.
