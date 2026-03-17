---
description: Brainstorming and decision framing for adapter-surface-closeout-2026-03-17.
---

# adapter-surface-closeout-2026-03-17 Brainstorming

## Problem

The repo now presents one supported runtime story: Rust backend authority plus a thin OpenClaw shell.

The remaining operator pain is no longer backend ownership drift. The remaining pain is shell-surface drift and residual local debt:

- README/runtime docs claim backend-native `distill` support, but the plugin exposes no distill tool or status surface.
- backend-owned debug recall routes exist, but the adapter/plugin surface does not expose them.
- `memoryReflection` still advertises local generation-era knobs even though `/new` and `/reset` now only enqueue backend jobs.
- `setwise-v2` is still shipped as a prompt-local seam, but the runtime mapping drops embeddings and source metadata, so its semantic branch is effectively dead in normal production flow.
- top-level `src/` and `package.json` still contain local RAG residue that obscures the actual authority boundary.

Success means the repo has one auditable answer to three questions:

- what the plugin shell actually supports today;
- which local seams are intentionally retained;
- which residual TypeScript/config/package artifacts must be removed, demoted, or explicitly downscoped.

## Scope

- freeze the exact adapter/plugin gaps discovered in the 2026-03-17 scan;
- decide which backend-native capabilities must gain plugin shell surfaces in this scope;
- decide which stale local reflection/config paths are removed versus retained as compatibility no-ops;
- define the supported runtime semantics for prompt-local `setwise-v2`;
- define the final cleanup target for residual TS helper files, package metadata, and README claims.

## Constraints

- `backend/` remains the only supported authority for persistence, recall, ranking, scope derivation, ACL, reflection execution, and distill execution.
- Ordinary `/v1/recall/*` DTOs must not be widened just to recover old TS-local heuristics.
- New operator/debug surfaces must stay behind the existing runtime principal boundary and should be gated behind `enableManagementTools` unless there is a strong reason not to.
- Config cleanup must not silently break deployed configs without an explicit compatibility story.
- This scope must not reintroduce local reflection generation, local scope selection, or local fallback memory authority.
- Validation must stay repo-native:
  - `npm test`
  - focused Node integration tests under `test/`
  - `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture` when backend-facing contracts or assumptions are touched.

## Options

### Option A: Documentation-only downscope

Shape:

- revise README/docs to say distill/debug shell surfaces are not yet exposed;
- keep current plugin/runtime code mostly unchanged;
- defer dead-code cleanup and config alignment.

Pros:

- lowest short-term risk;
- no public tool-surface churn.

Cons:

- leaves real dead-code and stale-config debt in place;
- keeps backend capabilities stranded behind an incomplete adapter layer;
- does not solve the maintenance confusion that caused this scan.

### Option B: Full shell-surface closeout plus narrow cleanup

Shape:

- add management-gated distill enqueue/status tools;
- add management-gated recall debug trace surface;
- remove or explicitly deprecate stale local reflection-generation config and helper code;
- lock `setwise-v2` to prompt-local semantics that work with the stable runtime DTO;
- clean residual TS/package/docs debt proved unused by the active runtime.

Pros:

- directly resolves the highest-signal gaps from the scan;
- keeps backend authority unchanged;
- gives operators real access to already-shipped backend-native capabilities.

Cons:

- touches multiple modules and tests;
- requires a precise compatibility policy for stale config keys;
- package/debt cleanup can spill into docs/tests if not staged carefully.

### Option C: Broader architecture reset

Shape:

- redesign plugin/public surfaces, config schema, and prompt-local seams all at once;
- reconsider whether `setwise-v2` or reflection prompt planning should remain local at all.

Pros:

- architecturally clean on paper.

Cons:

- too large for the concrete issues discovered in this scan;
- risks reopening already-frozen backend/shell boundaries;
- conflates debt cleanup with new architecture work.

## Decision

Select **Option B**.

This scope should close the shell/adapter gaps that are already backed by real Rust contracts, while narrowing or removing local artifacts that no longer participate in the supported runtime.

The implementation posture is:

- ship missing plugin surfaces where the backend contract is already real and stable;
- do not widen ordinary recall DTOs for legacy local heuristics;
- treat `setwise-v2` as a prompt-local post-selection seam over stable backend rows, not as a hidden reimplementation of backend ranking;
- expose distill as two management-gated tools:
  - `memory_distill_enqueue`
  - `memory_distill_status`
- expose debug recall trace as a separate management-gated tool:
  - `memory_recall_debug`
- prefer deprecation-plus-removal sequencing for stale config over silent hard breaks;
- keep `memoryReflection.agentId`, `maxInputChars`, `timeoutMs`, and `thinkLevel` parseable-but-ignored for this scope while Phase 3 removes their runtime effect and Phase 4 aligns docs/schema text.

## Risks

- adding distill/debug management tools can expose more operator power than intended if gating or redaction is sloppy;
- removing dead local reflection helpers from `index.ts` can accidentally delete utility code still used by tests or bootstrap flows;
- package/dependency cleanup can break `npm test` if imports are indirectly resolved through test-only code paths;
- docs may overclaim “fully solved” if phase execution stalls after the scope is created.

## Phase 1 Freeze Resolutions

- debug recall trace is frozen as a dedicated management tool: `memory_recall_debug`.
- distill is frozen as two management tools: `memory_distill_enqueue` and `memory_distill_status`.
- stale `memoryReflection.agentId`, `maxInputChars`, `timeoutMs`, and `thinkLevel` are frozen as parseable-but-ignored compatibility fields for this scope.
- `query-expander.ts`, `chunker.ts`, `src/reflection-store.ts`, and residual `package.json` local-RAG metadata are frozen as later-phase cleanup targets once import/use-site proof is collected.
