---
description: Contracts and scope freeze for completing the Rust-owned RAG pipeline.
---

# rust-rag-completion Contracts

## Context
The legacy local TypeScript RAG authority has been removed. The Rust backend under `backend/` must now become the real retrieval authority rather than an MVP contract stub.

## Findings
- `backend/` already provides the HTTP surface, auth middleware, LanceDB persistence seam, and contract tests.
- `backend/src/state.rs::recall_generic()` is still placeholder ranking (`contains(query)` → fixed score) and does not implement a real embedding + retrieval + rerank pipeline.
- `backend/src/state.rs::recall_reflection()` is also heuristic-only and does not yet prove backend-owned retrieval quality.
- Existing TypeScript code and docs still describe the intended authority model and preserve useful retrieval behavior references:
  - `src/embedder.ts`
  - `src/retriever.ts`
  - `src/store.ts`
  - `src/auto-recall-final-selection.ts`
  - `src/reflection-recall.ts`
  - `test/benchmark-fixtures.json`
  - `docs/archive/2026-03-15-architecture-reset/remote-memory-backend/*.md`

## Goals
1. Implement a real Rust generic RAG pipeline for `/v1/recall/generic`.
2. Reuse backend-owned config for embedding / rerank providers instead of local TS authority.
3. Preserve the stable runtime DTO boundary: no raw vector/BM25/rerank breakdown leakage on `/v1` data-plane responses.
4. Keep ACL / scope / principal ownership backend-side only.
5. Add meaningful verification proving the backend is no longer placeholder retrieval.

## Non-goals
- No rollback to local TypeScript RAG authority.
- No distributed worker architecture.
- No unrelated shell/plugin rewrites beyond what is needed to keep contracts aligned.
- No admin/control-plane expansion unless required by the retrieval implementation.

## Frozen runtime contracts
- `POST /v1/recall/generic` remains the generic recall authority endpoint.
- `POST /v1/recall/reflection` remains the reflection recall authority endpoint.
- Backend runtime identity continues to be derived from the trusted runtime headers plus actor envelope validation.
- Stable response rows may include only orchestration-facing ranking output (`score`) and stable metadata, not diagnostic component scores.
- Writes and recalls must continue to be caller-scoped by backend-owned principal / scope derivation.

## Required implementation shape
### Generic recall
The Rust backend must own these steps:
1. query normalization / validation
2. embedding generation through backend-owned provider config
3. candidate generation from LanceDB and any backend-side lexical path that is necessary for exact-match recall quality
4. backend-owned hybrid scoring / candidate merge
5. rerank or final ranking step when configured
6. deterministic top-k output after backend-side ranking

### Reflection recall
- Reflection recall may remain semantically distinct, but it must no longer be a pure stub path.
- It must respect the frozen high-level mode contract (`invariant-only` vs `invariant+derived`).
- If reflection retrieval depends on a later batch, the implementation must leave explicit seams and credible tests instead of silent stub-only behavior.

## Expected target files / modules
- `backend/src/config.rs`
- `backend/src/models.rs`
- `backend/src/state.rs`
- `backend/src/error.rs`
- `backend/tests/phase2_contract_semantics.rs`
- add focused backend tests if needed under `backend/tests/`
- docs under `docs/rust-rag-completion/`
- update top-level docs only if the implementation materially changes operator-facing behavior

## Verification contract
Minimum proof required before claiming completion:
- `cargo fmt --manifest-path backend/Cargo.toml`
- `cargo check --manifest-path backend/Cargo.toml`
- `cargo test --manifest-path backend/Cargo.toml`
- at least one focused test proving non-placeholder generic recall behavior
- at least one focused test or check proving provider/config failure behavior is handled explicitly
- updated phased checklist with evidence commands and remaining risks

## Rollback
If the full implementation cannot be completed safely in one batch, keep the repo in a compilable/testable state and leave the Rust path improved with explicit remaining gaps recorded in `docs/rust-rag-completion/task-plans/4phases-checklist.md`.

## Open questions
- Whether LanceDB alone is sufficient for acceptable lexical recall quality or whether a side lexical index is needed.
- Whether rerank should be implemented in the same batch or behind config-driven optional behavior.
- Whether reflection retrieval should share the same candidate generation core or remain a narrower specialized path.
