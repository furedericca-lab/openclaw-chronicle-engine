---
description: Phase 1 plan for turns-stage-distill-unification-2026-03-18.
---

# Phase 1 — Semantic contract freeze

## Goal
Freeze the post-refactor ownership model before touching implementation.

## Tasks
- Confirm that distill is the sole trajectory-derived write path.
- Confirm that `/new` / `/reset` no longer trigger knowledge generation.
- Confirm whether reflection remains read-only or is further reduced later.
- Enumerate exact code/docs/tests to delete or rewrite.

## Target files
- `docs/turns-stage-distill-unification-2026-03-18/*`
- `README.md`
- `README_CN.md`
- `index.ts`
- `src/backend-tools.ts`
- `src/backend-client/*`
- `test/*`
- `backend/tests/phase2_contract_semantics.rs`

## Verification
- `rg -n "reflection/source|reflection/jobs|everyTurns|session-lessons|memory_reflection_status" README* index.ts src test backend/tests`

## Done definition
- deletion list is explicit
- ownership wording is unambiguous
- no open semantic contradiction remains in planning docs
