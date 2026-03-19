---
description: Task list for governance-behavioral-closeout-2026-03-19 phase 3.
---

# Tasks: governance-behavioral-closeout-2026-03-19 Phase 3

## Input
- Canonical sources:
  - /root/verify/openclaw-chronicle-engine-governance-behavioral-closeout-2026-03-19/README.md
  - /root/verify/openclaw-chronicle-engine-governance-behavioral-closeout-2026-03-19/docs/archive/governance-behavioral-closeout-2026-03-19/governance-behavioral-closeout-2026-03-19-scope-milestones.md
  - /root/verify/openclaw-chronicle-engine-governance-behavioral-closeout-2026-03-19/docs/archive/governance-behavioral-closeout-2026-03-19/governance-behavioral-closeout-2026-03-19-technical-documentation.md
  - /root/verify/openclaw-chronicle-engine-governance-behavioral-closeout-2026-03-19/docs/archive/governance-behavioral-closeout-2026-03-19/governance-behavioral-closeout-2026-03-19-contracts.md

## Phase 3: Neutral Internal Behavioral-Guidance Naming

Goal: remove the most confusing remaining internal `reflection` naming without changing the frozen backend wire/storage contract.

Definition of Done:
- adapter/client/tool internals use behavioral-guidance terminology;
- backend helper/handler names move to behavioral-guidance wording where safe;
- removed config aliases are rejected explicitly.

Tasks:
- [x] T041 [Agentic] Canonicalize adapter/runtime internals.
  - DoD: `index.ts`, `src/backend-client/*`, `src/backend-tools.ts`, and `src/context/*` use behavioral-guidance naming and no longer expose dead reflection wrapper exports.
- [x] T042 [Backend] Rename safe backend internals.
  - DoD: `backend/src/lib.rs`, `backend/src/models.rs`, and `backend/src/state.rs` use behavioral-guidance helper/handler naming while keeping `/v1/recall/reflection` and persisted `reflection` fields unchanged.
- [x] T043 [QA] Update runtime tests to match canonical naming.
  - DoD: test helper paths/names, debug tool expectations, and config alias rejection coverage reflect the closeout semantics.

Evidence commands:
- `rg -n "autoRecallExcludeReflection|inheritance-only|inheritance\\+derived|recallReflection\\(" index.ts src test --glob '!docs/archive/**'`
- `rg -n "recall_behavioral_guidance|manual_behavioral_guidance_write_error|behavioralMode" backend/src src test`

Checkpoint:
- Phase 3 complete. Active code now speaks governance + behavioral guidance, with the remaining backend reflection contract documented rather than propagated through wrappers.
