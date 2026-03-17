## Context

`openclaw-chronicle-engine` now documents a strict ownership split:
- `backend/` owns persistence, retrieval, ranking, scope, ACL, reflection recall, and job execution;
- the plugin owns OpenClaw adapter wiring and prompt-time orchestration only.

A fresh `src/` scan still shows several boundary leaks and residual structure debts:
- one `src/` file (`noise-filter.ts`) has no runtime entry and is only exercised by tests;
- auto-recall and reflection-recall planners still perform candidate filtering/capping decisions locally after backend recall;
- `src/backend-tools.ts` still mixes remote memory adapter registration with self-improvement tool registration.

## Findings

- `src/noise-filter.ts` is not imported by `index.ts` or any active runtime `src/` module; it is only referenced by `test/noise-filter-chinese.mjs`.
- `src/context/auto-recall-orchestrator.ts` still applies local candidate shaping in `postProcessAutoRecallResults()`:
  - category allowlist filtering;
  - reflection exclusion;
  - max-age filtering;
  - max-entries-per-key filtering;
  - optional prompt-local `setwise-v2` post-selection.
- `src/context/reflection-prompt-planner.ts` still applies local reflection candidate shaping in the dynamic path:
  - kind filtering;
  - score threshold filtering;
  - local `slice(0, topK)` after backend recall.
- `src/recall-engine.ts` mixes session-local dedupe/exposure state with reusable candidate-pruning helpers used by the planners.
- `src/backend-tools.ts` currently imports and registers `self_improvement_*` tools even though they are governance/file-workflow surfaces, not remote memory adapter surfaces.

## Goals / Non-goals

Goals:
- remove or relocate the non-runtime `noise-filter.ts` residue;
- tighten the recall boundary so backend-visible candidate filtering semantics are backend-owned rather than plugin-owned;
- reduce adapter-surface sprawl by separating self-improvement registration from `src/backend-tools.ts`;
- preserve prompt-local orchestration that only affects injection timing, session-local suppression, or final prompt formatting.

Non-goals:
- no reintroduction of local-authority memory behavior;
- no change to the canonical requirement that `remoteBackend.enabled=true`;
- no broad redesign of self-improvement product scope unless required to separate registration seams cleanly.

## Target files / modules

- `src/noise-filter.ts`
- `test/noise-filter-chinese.mjs`
- optional `test/helpers/*` relocation target if `noise-filter` remains test-only
- `src/context/auto-recall-orchestrator.ts`
- `src/context/reflection-prompt-planner.ts`
- `src/recall-engine.ts`
- `src/backend-tools.ts`
- `src/self-improvement-tools.ts`
- `src/backend-client/types.ts`
- `src/backend-client/client.ts`
- `backend/src/*` as needed for new backend-side recall filter semantics
- `index.ts`
- `README.md`
- `README_CN.md`
- `docs/runtime-architecture.md`

## Constraints

- Keep backend authority explicit: plugin must not own retrieval/ranking/filter semantics that decide the authoritative candidate set.
- Local prompt-only shaping may remain only if it does not recreate retrieval/ranking authority.
- If backend routes need new request fields, evolve them additively within the existing `/v1` contract.
- Any retained prompt-local `setwise-v2` behavior must be justified as final prompt composition, not candidate authority.
- Prefer a phased implementation if discovery confirms this touches both TS adapter code and Rust backend API semantics.

## Verification plan

- `npm test`
- relevant Rust backend tests for any changed recall contract / filter semantics
- `rg -n "noise-filter|postProcessAutoRecallResults|minScore|maxEntriesPerKey|excludeReflection|registerSelfImprovement" index.ts src test README.md README_CN.md docs/runtime-architecture.md --glob '!docs/archive/**'`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/recall-boundary-tightening`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/recall-boundary-tightening README.md`

## Rollback

- Restore local planner-side filtering if backend-side recall contract changes prove behaviorally incompatible.
- Revert any self-improvement registration split if OpenClaw tool registration semantics require the current co-location.
- Restore `noise-filter.ts` to `src/` only if a real runtime path is discovered during implementation.

## Open questions

- Whether `setwise-v2` should remain as a prompt-local post-selection seam or move fully into backend recall semantics.
- Whether the backend should accept explicit generic/reflection recall filter fields or infer them from plugin config defaults alone.
- Whether self-improvement tools should stay in this plugin at all, or only be separated structurally from the memory adapter layer for now.

## Closeout

Status: completed and archived on 2026-03-17.

Outcome:
- removed runtime-dead `src/noise-filter.ts` and relocated the retained reference helper under `test/helpers/`;
- moved generic/reflection candidate-filter semantics into backend request/response contracts and backend-side filtering execution;
- kept only prompt-local final shaping in the plugin, including retained `setwise-v2` post-selection;
- separated self-improvement tool registration from the remote memory adapter registration surface.

Verification evidence:
- `npm test`
- `cargo test generic_recall_applies_backend_owned_filter_fields -- --nocapture`
- `cargo test reflection_recall_applies_include_kinds_filter -- --nocapture`
