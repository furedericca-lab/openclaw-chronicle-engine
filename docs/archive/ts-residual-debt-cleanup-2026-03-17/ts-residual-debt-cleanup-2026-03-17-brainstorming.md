---
description: Brainstorming and decision framing for ts-residual-debt-cleanup-2026-03-17.
---

# ts-residual-debt-cleanup-2026-03-17 Brainstorming

## Problem

The repo no longer has a supported local-authority runtime, but several TypeScript helper modules still look like remnants of the old retrieval stack:

- `src/recall-engine.ts`
- `src/auto-recall-final-selection.ts`
- `src/final-topk-setwise-selection.ts`
- `src/reflection-recall.ts`
- `src/reflection-recall-final-selection.ts`
- `src/adaptive-retrieval.ts`

The main risk is no longer hidden authority drift. The main risk is maintenance confusion:

- some files are production-critical prompt-local seams;
- some files are generic utilities with authority-sounding names;
- some files may now be test/reference-only but still live under top-level `src/`.

## Scope

- audit current use-sites of retained TS recall/reflection helpers;
- classify each helper as:
  - production prompt-local seam,
  - production utility with naming/location debt,
  - test/reference-only helper,
  - removable dead code;
- define the cleanup scope needed to reduce ambiguity without reintroducing local authority.

## Constraints

- no local-authority runtime may be reintroduced;
- backend remains authoritative for recall/ranking/ACL/scope;
- prompt-time orchestration may remain local when it does not change backend authority;
- cleanup should prefer clarity of ownership over cosmetic churn.

## Options

### Option A: Leave everything in place and rely on docs only

Pros:

- zero code churn;
- no test movement risk.

Cons:

- keeps high confusion for future maintainers;
- does not reduce `src/` noise;
- the same “is this old TS debt still alive?” question will keep returning.

### Option B: Delete any TS helper that is not on the hottest production path

Pros:

- aggressive reduction of apparent debt;
- quickly shrinks root-level `src/`.

Cons:

- too risky without dependency reshaping;
- test/reference helpers may still carry useful algorithm fixtures;
- can easily turn cleanup into behavioral refactor.

### Option C: Audit first, then perform a narrow debt cleanup in phases

Track 1:

- freeze ownership with file-by-file evidence.

Track 2:

- relocate or rename test/reference-only helpers.

Track 3:

- rename or relocate production prompt-local helpers whose names still imply backend authority.

Pros:

- maximizes clarity while minimizing accidental behavior change;
- compatible with current remote-authority architecture;
- gives explicit delete/move/keep decisions.

Cons:

- slower than a blind delete pass;
- requires discipline not to mix readability cleanup with feature work.

## Decision

Select **Option C**.

This scope should not start by deleting code. It should first freeze which TS modules are:

- genuinely required at runtime;
- runtime-local but poorly named;
- test/reference-only and candidates to move out of top-level `src/`;
- truly dead and removable.

## Risks

- helper movement can break tests that use `jiti("../src/...")`;
- renaming production prompt-local helpers can create churn in docs and README examples;
- some helpers may look dead in production but still encode expectations that matter for regression fixtures.

## Open Questions

- should `src/reflection-recall.ts` and `src/reflection-recall-final-selection.ts` move to `test/helpers/`, `src/experimental/`, or remain in `src/` with explicit comments;
- should `src/auto-recall-final-selection.ts` be renamed to emphasize prompt-local usage, for example `prompt-local-auto-recall-selection.ts`;
- should `src/recall-engine.ts` be split so orchestration/session-state helpers are separated from text-key normalization utilities.
