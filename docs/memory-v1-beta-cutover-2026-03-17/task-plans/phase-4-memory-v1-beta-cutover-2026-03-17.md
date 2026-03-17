---
description: Task list for memory-v1-beta-cutover-2026-03-17 phase 4.
---

# Tasks: memory-v1-beta-cutover-2026-03-17 Phase 4

## Input
- Canonical sources:
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/package.json
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/openclaw.plugin.json
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/CHANGELOG.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/README.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/README_CN.md

## Canonical architecture / Key constraints
- all public version surfaces must match exactly;
- release docs must reflect the new-project baseline rather than migration compatibility;
- this phase is the release gate and must include regression and hygiene checks.

## Phase 4: Release Closeout
Goal: finish version/reset semantics and final verification.

Definition of Done: all public version surfaces report `1.0.0-beta.0`, docs are aligned, and regression/hygiene checks pass.

Tasks:
- [x] T061 [Config] Update `package.json`, `openclaw.plugin.json`, lockfile metadata if needed, and `CHANGELOG.md` to `1.0.0-beta.0`.
  - DoD: package/plugin/changelog versions are consistent and intentional.
- [x] T062 [P] [Docs] Update active docs to present the repo as the new-project beta baseline rather than a migration bridge.
  - DoD: README text, examples, and release notes no longer rely on legacy-compatibility framing.
- [x] T063 [QA] Run release-gate verification.
  - DoD: `npm test`, targeted `rg` scans, `doc_placeholder_scan`, and `post_refactor_text_scan` pass, and any remaining risks are recorded in the checklist.

Checkpoint: the repo is ready to ship as `1.0.0-beta.0`.

## Evidence

- `package.json`, `package-lock.json`, and `openclaw.plugin.json` now report `1.0.0-beta.0`.
- `CHANGELOG.md` has a new `1.0.0-beta.0` entry describing the cutover.
- Active docs now describe the repo as a clean post-migration baseline rather than a compatibility bridge.

## Verification Commands

- `npm test`
- `jq empty openclaw.plugin.json`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/memory-v1-beta-cutover-2026-03-17`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/memory-v1-beta-cutover-2026-03-17 README.md`
- `rg -n "sessionMemory.enabled|sessionMemory.messageCount|memoryReflection\\.agentId|memoryReflection\\.maxInputChars|memoryReflection\\.timeoutMs|memoryReflection\\.thinkLevel|1\\.1\\.0-beta\\.6|src/query-expander\\.ts|src/reflection-store\\.ts|\\[TODO\\]" package.json package-lock.json openclaw.plugin.json README.md README_CN.md index.ts test src`

## Dependencies & Execution Order
- Depends on Phases 1-3.
- T061 should land before final release-note wording is frozen in T062.
