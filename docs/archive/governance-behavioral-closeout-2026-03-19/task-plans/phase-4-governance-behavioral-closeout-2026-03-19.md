---
description: Task list for governance-behavioral-closeout-2026-03-19 phase 4.
---

# Tasks: governance-behavioral-closeout-2026-03-19 Phase 4

## Input
- Canonical sources:
  - /root/verify/openclaw-chronicle-engine-governance-behavioral-closeout-2026-03-19/README.md
  - /root/verify/openclaw-chronicle-engine-governance-behavioral-closeout-2026-03-19/docs/archive/governance-behavioral-closeout-2026-03-19/governance-behavioral-closeout-2026-03-19-scope-milestones.md
  - /root/verify/openclaw-chronicle-engine-governance-behavioral-closeout-2026-03-19/docs/archive/governance-behavioral-closeout-2026-03-19/governance-behavioral-closeout-2026-03-19-technical-documentation.md
  - /root/verify/openclaw-chronicle-engine-governance-behavioral-closeout-2026-03-19/docs/archive/governance-behavioral-closeout-2026-03-19/governance-behavioral-closeout-2026-03-19-contracts.md

## Phase 4: Archive and Verification Closeout

Goal: archive the superseded scope, run the required verification commands, and record final evidence in the scope docs.

Definition of Done:
- previous unification scope is moved under `docs/archive/`;
- docs indexes/runtime docs reflect the new archive disposition;
- verification commands and outcomes are recorded;
- placeholder and residual scans are clean.

Tasks:
- [x] T061 [Docs] Archive the superseded scope.
  - DoD: `docs/autorecall-governance-unification-2026-03-18/` is moved to `docs/archive/` and the docs index/archive index are updated.
- [x] T062 [QA] Run verification commands and record outcomes.
  - DoD: JS tests, backend cargo verification, placeholder scan, and residual scan are completed or any blocking environment issue is documented with exact details.
- [x] T063 [Docs] Close out the scope docs with final evidence.
  - DoD: `technical-documentation.md`, `scope-milestones.md`, and `4phases-checklist.md` reflect final semantics, changed files, and archive decisions.

Evidence commands:
- `npm ci`
- `node --test --test-name-pattern='.' test/governance-tools.test.mjs test/auto-recall-behavioral.test.mjs test/config-session-strategy-cutover.test.mjs test/remote-backend-shell-integration.test.mjs`
- `npm test`
- `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/governance-behavioral-closeout-2026-03-19`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/governance-behavioral-closeout-2026-03-19 README.md README_CN.md docs/runtime-architecture.md docs/README.md docs/archive-index.md`
- `rg -n "self_improvement_|selfImprovement|\\.learnings|reflection-prompt-planner|reflection-error-signals|autoRecallExcludeReflection|inheritance-only|inheritance\\+derived" docs --glob '!docs/archive/**' --glob '!docs/archive/governance-behavioral-closeout-2026-03-19/**'`
- `rg -n "self_improvement_|selfImprovement|\\.learnings|reflection-prompt-planner|reflection-error-signals|autoRecallExcludeReflection|inheritance-only|inheritance\\+derived" src test index.ts openclaw.plugin.json README.md README_CN.md docs/runtime-architecture.md docs/README.md docs/archive-index.md --glob '!docs/archive/**'`

Checkpoint:
- Phase 4 closed on 2026-03-19 after targeted JS tests, the full JS suite, backend cargo verification, placeholder scan, post-refactor scan, and residual naming scans were all recorded.
