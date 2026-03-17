---
description: Implementation research notes for memory-v1-beta-cutover-2026-03-17.
---

# memory-v1-beta-cutover-2026-03-17 Implementation Research Notes

## Baseline (Current State)

- package and plugin metadata still report `1.1.0-beta.6`;
- config parsing in `index.ts` still maps `sessionMemory.*` to current fields and still parses deprecated `memoryReflection.*` fields into `deprecatedIgnoredFields`;
- `openclaw.plugin.json`, `README.md`, and `README_CN.md` still document those removed-by-intent compatibility behaviors;
- `src/query-expander.ts` and `src/reflection-store.ts` remain in top-level `src/`, while only tests import them;
- `src/self-improvement-tools.ts` contains literal placeholder checklist strings in user-facing generated output.

## Gap Analysis

1. Release-semantic gap:
   - the repo claims a post-migration architecture but still publishes under a continuing `1.1.0-beta.6` line.
2. Config-contract gap:
   - removed-in-practice migration fields are still accepted at parse time.
3. Layout / clarity gap:
   - test-only reference helpers still look like runtime modules.
4. User-visible polish gap:
   - placeholder checklist text still ships in self-improvement outputs.

## Candidate Designs and Trade-offs

### Hard-remove legacy fields in one cut

Pros:

- clearest contract;
- smaller long-term parser surface.

Cons:

- breaks any remaining old configs immediately.

### Stage removal with one more warning release

Pros:

- lower operator shock.

Cons:

- conflicts with the requested “new project” framing;
- creates another round of throwaway compatibility work.

### Keep helpers under `src/` but rename them

Pros:

- smaller move diff;
- less import rewrite churn.

Cons:

- still leaves test-only artifacts in runtime module territory.

### Move helpers under `test/helpers/`

Pros:

- strongest repo-layout clarity;
- aligns file ownership with actual usage.

Cons:

- requires test import rewrites and possible fixture path cleanup.

## Selected Design

- make this a breaking beta cutover;
- remove legacy config compatibility parsing instead of extending warnings;
- move test-only helper modules out of top-level runtime space if possible, with rename-only fallback only if movement becomes unexpectedly noisy;
- clean user-visible placeholder checklist text within the same scope because the touched tests and docs are already nearby.

## Ownership Boundary Notes

- backend authority is already settled and remains unchanged;
- the main work is in plugin parser/schema/docs/tests and repo layout hygiene;
- archived scope docs remain historical reference only and must not drive active compatibility policy.

## Parity / Migration Notes

- no historical config aliasing must survive just for migration nostalgia;
- current prompt-local seams remain valid and are not targets unless a touched helper turns out to be production-bound;
- archived docs may mention older compatibility layers, but active docs should not.

## Residue / Debt Disposition Notes

- `sessionMemory.*` support should be removed from parser, schema, tests, and README;
- deprecated local reflection-generation fields should be removed from parser/schema/docs/tests, not merely warned;
- `src/query-expander.ts` and `src/reflection-store.ts` should move under `test/helpers/` if import-proof remains test-only;
- placeholder checklist strings in shipped user-facing templates should be replaced with concrete neutral wording or removed.

## Validation Plan

- targeted parser/config regression tests;
- Node test suite for helper relocation and self-improvement output;
- `rg` scans proving no remaining legacy config acceptance/docs in active files;
- version consistency check across `package.json`, `openclaw.plugin.json`, and `CHANGELOG.md`.

## Risks and Assumptions

- assumes beta consumers can tolerate a hard config break;
- assumes helper files are truly test-only and have no hidden runtime import path;
- assumes version reset to `1.0.0-beta.0` is intended as product-line semantics, not semver continuity.
