---
description: API and schema contracts for ts-residual-debt-cleanup-2026-03-17.
---

# ts-residual-debt-cleanup-2026-03-17 Contracts

## API Contracts

This scope should not change any public backend API contract.

Contract rule:

- cleanup may rename, move, annotate, or delete local TS helpers only if runtime behavior at the plugin/backend contract boundary remains unchanged.

## Shared Types / Schemas

Ownership classification contract:

- a helper may be called `production prompt-local` only if it does not own storage, retrieval authority, ACL, or backend-facing contract semantics;
- a helper may be called `test/reference-only` only if no production import path remains after audit;
- a helper may be called `removable` only if no production or test import path remains and no scope doc depends on it as preserved reference.

## Event and Streaming Contracts

Import-boundary contract:

- `src/context/*` may depend on prompt-local helpers;
- production runtime code must not depend on helpers classified as test/reference-only;
- after cleanup, test/reference helpers live under `test/helpers/` rather than top-level production-oriented `src/` roots.

## Error Model

Audit acceptance contract:

- if evidence is ambiguous, classify the file as “retain pending proof” rather than deleting it;
- no cleanup batch may rely on guesswork about dynamic imports or historical intent.

## Validation and Compatibility Rules

- `npm test` is required for any implementation cleanup batch;
- docs for this scope must record exact file disposition decisions before file movement begins;
- README explanations about “old TS removed vs prompt-local retained” must remain consistent with the final classification.
