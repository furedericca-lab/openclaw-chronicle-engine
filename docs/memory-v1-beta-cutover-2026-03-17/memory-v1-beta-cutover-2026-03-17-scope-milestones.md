---
description: Scope boundaries and milestones for memory-v1-beta-cutover-2026-03-17.
---

# memory-v1-beta-cutover-2026-03-17 Scope and Milestones

## In Scope

- reset package/plugin version to `1.0.0-beta.0`;
- remove migration-only config compatibility for legacy session and deprecated reflection-generation fields;
- update active schema/docs/tests to the new no-compatibility baseline;
- relocate or sharply reclassify test-only helper files that currently live under top-level `src/`;
- remove explicit placeholder checklist text from shipped self-improvement output.

## Out of Scope

- backend REST contract changes;
- new memory/retrieval features;
- major refactors of prompt-local recall algorithms;
- archive-wide historical doc rewriting outside the new active scope.

## Milestones

### Milestone 1: Breaking Beta Contract Freeze

Acceptance:

- version target is frozen as `1.0.0-beta.0`;
- removed config fields are explicitly enumerated;
- helper-residue disposition rule is frozen;
- checklist and phase plans are concrete enough to begin code changes.

### Milestone 2: Legacy Config Removal

Acceptance:

- parser no longer accepts `sessionMemory.*` or deprecated local reflection-generation fields;
- schema/help/README no longer advertise those fields;
- tests assert rejection or absence instead of mapping and ignored-field warnings.

### Milestone 3: Helper and Template Residue Cleanup

Acceptance:

- `src/query-expander.ts` and `src/reflection-store.ts` no longer present as ambiguous runtime-top-level modules, or they are renamed/relocated with explicit test-only ownership;
- self-improvement output no longer emits placeholder checklist text;
- imports and tests are updated accordingly.

### Milestone 4: Release Surface Closeout

Acceptance:

- package/plugin/changelog versions are consistent at `1.0.0-beta.0`;
- active docs present the repo as the new-project baseline, not a migration bridge;
- regression and hygiene scans pass.

## Dependencies

- Milestone 2 depends on Milestone 1 contracts;
- Milestone 3 depends on Milestone 2 parser/schema/docs decisions to avoid churn;
- Milestone 4 is the release gate and depends on Milestones 1-3.

## Exit Criteria

- active codebase contains no accepted legacy config aliasing for removed fields;
- active docs do not tell users those removed fields are still supported;
- top-level runtime module layout no longer misrepresents test-only helpers;
- shipped metadata is consistently `1.0.0-beta.0`;
- selected regression tests and doc hygiene scans pass.

## Archive / Handoff Note

- once implemented and verified, this scope should be archived under `docs/archive/memory-v1-beta-cutover-2026-03-17`.
