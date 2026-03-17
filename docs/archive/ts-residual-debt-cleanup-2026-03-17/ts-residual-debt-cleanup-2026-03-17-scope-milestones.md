---
description: Scope boundaries and milestones for ts-residual-debt-cleanup-2026-03-17.
---

# ts-residual-debt-cleanup-2026-03-17 Scope and Milestones

## In Scope

- audit retained TypeScript recall/reflection helper modules that still look like migration residue;
- freeze a file-by-file classification for keep / move / rename / remove decisions;
- plan a cleanup that reduces top-level `src/` ambiguity without changing backend authority;
- ensure future cleanup preserves current runtime behavior and test coverage.

## Out of Scope

- changing backend recall/ranking behavior;
- reintroducing local-authority runtime behavior;
- changing public backend contracts to accommodate TS cleanup;
- broad refactors unrelated to TS residual debt clarity.

## Milestones

### Milestone 1 — Residual helper audit freeze

Acceptance gate:

- each target helper has concrete import/use-site evidence;
- production prompt-local helpers are separated from test/reference-only helpers in docs;
- the audit clearly states whether any true dead code was found.

### Milestone 2 — Reference/helper relocation plan

Acceptance gate:

- docs define which files should move out of top-level `src/`;
- tests and path updates required by that move are enumerated.

Status:

- completed: `src/reflection-recall.ts` and `src/reflection-recall-final-selection.ts` were relocated to `test/helpers/` and test imports were updated.

### Milestone 3 — Naming and seam-clarity cleanup plan

Acceptance gate:

- docs define which retained production helpers should be renamed, commented, or split for clarity;
- a checklist exists for validating that cleanup without changing authority semantics.

Status:

- completed: `src/auto-recall-final-selection.ts` and `src/final-topk-setwise-selection.ts` were renamed to prompt-local equivalents and runtime/test imports were updated.

## Dependencies

- Milestone 1 blocks all cleanup work.
- Milestone 2 depends on Milestone 1 classification.
- Milestone 3 depends on Milestone 1 and should use Milestone 2 outcomes where file moves affect naming decisions.

## Exit Criteria

- the repo has one canonical audit scope for TS residual debt under `docs/archive/ts-residual-debt-cleanup-2026-03-17/`;
- future cleanup work can proceed without re-auditing the same helper set;
- maintainers can tell which TS helpers are runtime-local seams and which are cleanup candidates.
