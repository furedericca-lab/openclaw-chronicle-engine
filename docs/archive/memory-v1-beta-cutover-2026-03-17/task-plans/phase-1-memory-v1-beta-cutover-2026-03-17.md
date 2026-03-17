---
description: Task list for memory-v1-beta-cutover-2026-03-17 phase 1.
---

# Tasks: memory-v1-beta-cutover-2026-03-17 Phase 1

## Input
- Canonical sources:
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/memory-v1-beta-cutover-2026-03-17/memory-v1-beta-cutover-2026-03-17-scope-milestones.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/memory-v1-beta-cutover-2026-03-17/memory-v1-beta-cutover-2026-03-17-technical-documentation.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/memory-v1-beta-cutover-2026-03-17/memory-v1-beta-cutover-2026-03-17-contracts.md

## Canonical architecture / Key constraints
- this phase freezes the breaking cutover contract before code edits begin;
- no implementation should preserve removed-field compatibility beyond this phase;
- version target must be locked as `1.0.0-beta.0`.

## Phase 1: Contract Freeze
Goal: freeze the new-project beta contract and exact removal list.

Definition of Done: version target, removed fields, helper-residue rule, and release gate are explicit enough to implement without rediscovery.

Tasks:
- [x] T001 [Docs] Freeze the exact removed config field set and the version target in scope docs.
  - DoD: contracts, milestones, and technical docs all agree on `1.0.0-beta.0` and the removed legacy field list.
- [x] T002 [P] [Docs] Freeze the accepted disposition of `src/query-expander.ts`, `src/reflection-store.ts`, and shipped placeholder checklist text.
  - DoD: scope docs say whether these files move, rename, or remain with explicit test-only ownership, and whether placeholder text is removed or rewritten.
- [x] T003 [QA] Record the verification matrix for parser, doc, helper-layout, and version consistency checks.
  - DoD: concrete commands and target test files are listed in the scope docs and checklist.

Checkpoint: Phase 1 docs are concrete enough to start breaking code changes in Phase 2.

## Evidence

- Contracts, milestones, technical documentation, and checklist all freeze `1.0.0-beta.0` as the target line.
- Removed fields and helper-residue disposition are explicitly enumerated in the active scope docs.

## Dependencies & Execution Order
- Phase 1 blocks all others.
- T001-T003 may overlap only when edits do not touch the same doc file.
