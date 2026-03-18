---
description: Phase 4 plan for turns-stage-distill-unification-2026-03-18.
---

# Phase 4 — Docs and residual cleanup

## Goal
Align all public and internal docs with the new semantics and ensure no stale reflection-generation promises remain.

## Tasks
- Rewrite README sections that document `/new` / `/reset` reflection generation.
- Rewrite distill wording from rejected map-stage/token-chunk framing to turns-stage lesson extraction.
- Document the frozen mode split:
  - `session-lessons` for lesson/cause/fix/prevention/stable decision/durable practice
  - `governance-candidates` for promotion-oriented governance outputs
  - `follow-up-focus` / `next-turn-guidance` as downgraded distill artifact subtypes
- Remove stale management/debug references for deleted reflection job surfaces.
- Remove any wording that implies rollback compatibility for deleted reflection-generation behavior.
- Run placeholder/residual scans and fix leftovers.

## Target files
- `README.md`
- `README_CN.md`
- `docs/runtime-architecture.md`
- `docs/turns-stage-distill-unification-2026-03-18/*`

## Verification
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/turns-stage-distill-unification-2026-03-18`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/turns-stage-distill-unification-2026-03-18 README.md`
- `rg -n "reflection/source|reflection/jobs|/new 或 /reset|map-stage|token-chunk" README* docs index.ts src test backend/tests`

## Done definition
- docs describe one write path and one cadence trigger model
- no stale command-triggered reflection-generation wording remains
- residual scans are clean
