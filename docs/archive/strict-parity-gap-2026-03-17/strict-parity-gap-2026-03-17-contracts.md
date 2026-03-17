# strict-parity-gap-2026-03-17 Contracts

## API / behavior contracts

This scope is primarily a parity-closure scope, but the following behavioral contracts are frozen for implementation work.

### Contract A: strict gap classification

An item may be listed as a strict remaining gap only if:

- the historical TS stack demonstrably owned or exposed that behavior; and
- the current repo does not yet provide an equivalent or explicitly justified Rust-native / remote-native replacement.

An item must not be listed as a gap if:

- it is already implemented and tested in `backend/src/*`; or
- it is intentionally retained prompt-local orchestration and not authority-layer behavior; or
- the historical TS capability has already been replaced by an acceptable Rust-native / remote-native implementation.

### Contract B: retrieval traceability

If strict parity work adds traceability features, they must provide:

- stable structured fields for stage name, fallback reason, candidate/result counts, and finalization decisions;
- no leakage into ordinary `/v1/recall/*` response rows;
- explicit ownership and authorization semantics.

The implementation does not need to mirror historical TS trace object shapes exactly if the Rust-native replacement preserves equivalent debugging capability.

### Contract C: retained TS helper ownership

For each retained helper under `src/*.ts` that participates in recall/final selection:

- docs must classify whether it is backend-parity debt or prompt-local orchestration;
- implementation must not leave that ownership ambiguous after closure;
- if a helper remains local, tests/docs must show that it does not reintroduce backend-authority logic and is an acceptable parity implementation under the current architecture.

Phase 1 frozen ownership decisions:

- `src/auto-recall-final-selection.ts` is an acceptable prompt-local post-selection seam when used only by `src/context/auto-recall-orchestrator.ts` after backend recall returns rows, and when backend recall DTO/authority semantics remain unchanged.
- `src/final-topk-setwise-selection.ts` is an acceptable local utility while it remains exclusively downstream of prompt-local planners/tests.
- `src/recall-engine.ts`, `src/context/*`, and `src/adaptive-retrieval.ts` are frozen as acceptable local prompt/runtime orchestration helpers unless later evidence shows that they alter backend-authority semantics.
- `src/reflection-recall.ts` and `src/reflection-recall-final-selection.ts` are frozen as retained test/reference helpers, not active production debt, unless production imports expand.

Phase 2-3 finalized ownership decisions:

- backend traceability parity is satisfied by explicit debug-scoped routes `/v1/debug/recall/generic` and `/v1/debug/recall/reflection`, returning trace data outside ordinary `/v1/recall/*` DTO rows;
- debug trace routes reuse runtime bearer auth plus actor principal matching, so trace visibility stays principal-scoped rather than introducing a separate weakly-guarded surface;
- `src/auto-recall-final-selection.ts` is no longer treated as backend debt by default because current evidence shows it only performs prompt-local shortlist shaping over backend-owned rows and does not change backend request/response contracts.

## Shared schema / type ownership

Current stable ownership:

- runtime recall DTOs: `backend/src/models.rs`, mirrored in `src/backend-client/types.ts`
- prompt-local injected block rendering: `src/context/*`

Potential strict-parity additions:

- internal retrieval trace schema owned by `backend/src/*`
- debug trace response schema owned by backend docs/tests and excluded from ordinary runtime DTOs

## Validation rules and compatibility policy

- no breaking changes to existing `/v1` data-plane responses unless the scope explicitly introduces a versioned contract;
- any trace/admin/debug surface must be additive;
- current remote-only runtime invariants remain mandatory;
- deleted local-authority files must stay deleted.

## Security-sensitive fields and redaction rules

- trace surfaces must not expose bearer tokens, raw secret env substitutions, or unauthorized principal data;
- diagnostic payloads must truncate upstream/provider error bodies to bounded sizes;
- persisted trace records, if introduced, must redact or bound sensitive text fields where necessary.
