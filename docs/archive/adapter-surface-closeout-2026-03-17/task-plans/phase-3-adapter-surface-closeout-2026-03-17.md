---
description: Task list for adapter-surface-closeout-2026-03-17 phase 3.
---

# Tasks: adapter-surface-closeout-2026-03-17 Phase 3

## Input
- Canonical sources:
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/README.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/adapter-surface-closeout-2026-03-17/adapter-surface-closeout-2026-03-17-scope-milestones.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/adapter-surface-closeout-2026-03-17/adapter-surface-closeout-2026-03-17-technical-documentation.md
  - /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/adapter-surface-closeout-2026-03-17/adapter-surface-closeout-2026-03-17-contracts.md

## Canonical architecture / Key constraints
- Keep architecture aligned with adapter-surface-closeout-2026-03-17 scope docs and contracts.
- Keep provider/runtime/channel boundaries unchanged unless explicitly in scope.
- Keep security and test gates in Definition of Done.
- Cleanup must not reintroduce local reflection generation or local authority.
- `setwise-v2` must be aligned with the actual runtime row shape.

## Format
- [ID] [P?] [Component] Description
- [P] means parallelizable.
- Valid components: Backend, Frontend, Agentic, Docs, Config, QA, Security, Infra.
- Every task must have a clear DoD.

## Phase 3: Local Residual Cleanup and Semantic Alignment
Goal: Remove dead local reflection-generation residue and make prompt-local setwise semantics honest and test-backed.

Definition of Done: All phase tasks are implemented, tested, and evidenced with commands and outputs.

Tasks:
- [x] T041 [Agentic] Remove or demote dead local reflection-generation runtime helpers from `index.ts`.
  - DoD: unsupported local reflection-generation functions/imports are removed or isolated from the supported runtime path, while `/new` and `/reset` continue to enqueue backend reflection jobs and relevant tests stay green.
- [x] T042 [Config] Align `memoryReflection` schema and parse behavior with the supported runtime.
  - DoD: `openclaw.plugin.json` and `parsePluginConfig` agree on the status of `agentId`, `maxInputChars`, `timeoutMs`, and `thinkLevel`, with either deprecation/ignored semantics or an explicit removal/migration note.
- [x] T043 [P] [QA] Align `setwise-v2` with actual ordinary recall inputs.
  - DoD: `src/context/auto-recall-orchestrator.ts`, `src/prompt-local-auto-recall-selection.ts`, and `src/prompt-local-topk-setwise-selection.ts` no longer rely on nonexistent ordinary-row embeddings/source metadata in the supported runtime contract, and tests document the intended lexical/coverage behavior.
- [x] T044 [Security] Verify cleanup does not widen authority or principal behavior.
  - DoD: cleanup preserves existing runtime identity requirements and does not add local scope reconstruction or local fallback ranking behavior.

Checkpoint: Phase 3 artifacts are updated, verified, and recorded in `4phases-checklist.md` before next phase starts.

## Dependencies & Execution Order
- Phase 1 blocks all others.
- Phase 3 depends on completion of phases 1-2.
- Tasks marked [P] within this phase may run concurrently only when they do not touch the same files.
