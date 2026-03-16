---
description: Execution and verification checklist for rust-rag-completion 4-phase plan.
---

# Phases Checklist: rust-rag-completion

## Input
- `docs/rust-rag-completion/rust-rag-completion-contracts.md`
- `docs/rust-rag-completion/rust-rag-completion-scope-milestones.md`
- `backend/`
- TS reference modules under `src/`

## Rules
- Use this file as the single progress and audit hub.
- Update status, evidence commands, and blockers after each implementation batch.
- Do not mark a phase complete without evidence.
- Do not claim completion while generic recall remains placeholder scoring.

## Global Status Board
| Phase | Status | Completion | Health | Blockers |
|---|---|---|---|---|
| 1 | Ready | 100% | Good | 0 |
| 2 | Completed | 100% | Good | 0 |
| 3 | Completed | 100% | Good | 0 |
| 4 | Completed | 100% | Good | 0 |

## Phase Entry Links
1. [phase-1-rust-rag-completion.md](/root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/task-plans/phase-1-rust-rag-completion.md)
2. [phase-2-rust-rag-completion.md](/root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/task-plans/phase-2-rust-rag-completion.md)
3. [phase-3-rust-rag-completion.md](/root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/task-plans/phase-3-rust-rag-completion.md)
4. [phase-4-rust-rag-completion.md](/root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/task-plans/phase-4-rust-rag-completion.md)

## Phase Execution Records

### 2026-03-16 bootstrap
- Phase: 1
- Batch date: 2026-03-16
- Completed tasks:
  - confirmed `backend/` compiles and tests pass as MVP baseline
  - confirmed generic recall remains placeholder-ranked and therefore incomplete
  - created `docs/rust-rag-completion/*` as the active execution scope
- Evidence commands:
  - `cargo check --manifest-path backend/Cargo.toml`
  - `cargo test --manifest-path backend/Cargo.toml`
  - `rg -n "recall_generic|recall_reflection|embed|bm25|rerank" backend/src src docs -S`
- Issues/blockers:
  - real Rust retrieval authority not yet implemented
- Resolutions:
  - start a dedicated Codex implementation batch against this scope
- Checkpoint confirmed: yes

### 2026-03-16 rust-owned generic RAG implementation
- Phase: 2-4
- Batch date: 2026-03-16
- Completed tasks:
  - replaced `backend/src/state.rs::recall_generic()` placeholder substring scoring with a backend-owned multi-stage retrieval pipeline:
    - query normalization/tokenization
    - backend embedding generation (deterministic hashing embedder configured in Rust backend config)
    - lexical BM25-like scoring
    - hybrid candidate fusion
    - optional backend rerank blend
    - recency/importance/length/time-decay transforms
    - deterministic final ordering + top-k truncation
  - routed `backend/src/state.rs::recall_reflection()` through the same real ranking core (while preserving mode semantics for `invariant-only` vs `invariant+derived`)
  - extended Rust backend config ownership for retrieval/provider runtime wiring in `backend/src/config.rs`:
    - added `[providers]` and `[retrieval]` validation and defaults
    - explicit validation failure for unsupported/invalid embedding+rerank config
  - added focused verification tests in `backend/tests/phase2_contract_semantics.rs`:
    - `generic_recall_prefers_real_signal_over_placeholder_ordering`
    - `invalid_embedding_dimensions_config_is_rejected`
  - preserved stable `/v1` recall DTO boundary (no raw `vectorScore`/`bm25Score`/`rerankScore` fields in responses)
- Evidence commands:
  - `cargo fmt --manifest-path backend/Cargo.toml`
  - `cargo check --manifest-path backend/Cargo.toml`
  - `cargo test --manifest-path backend/Cargo.toml`
  - `cargo test --manifest-path backend/Cargo.toml generic_recall_prefers_real_signal_over_placeholder_ordering -- --exact --nocapture`
- Evidence results:
  - `cargo fmt`: pass
  - `cargo check`: pass (non-blocking pre-existing warning: `ErrorCode::RateLimited` is never constructed)
  - `cargo test`: pass (18 tests passed, 0 failed)
  - focused generic-recall test: pass
- Issues/blockers:
  - reference files `src/embedder.ts`, `src/retriever.ts`, `src/store.ts` are absent in current worktree; historical behavior was recovered from git history and remaining live references (`src/auto-recall-final-selection.ts`, `src/reflection-recall.ts`, docs/archive)
- Resolutions:
  - implemented Rust-native retrieval semantics directly in `backend/src/state.rs` without restoring local TS authority
  - captured config-driven seam for future non-hashing embedding/rerank providers in Rust-only backend config
- Residual risks:
  - current embedding provider is deterministic local hashing (`providers.embedding.provider = "hashing"`); external model-backed embedding/rerank providers remain deferred
  - retrieval currently scores over caller-scoped rows in-memory rather than using LanceDB native vector/FTS indexes for candidate generation
- Checkpoint confirmed: yes

### 2026-03-16 continuation (TS full RAG authority port batch)
- Phase: 2-4
- Batch date: 2026-03-16
- Completed tasks:
  - recovered old TS retrieval authority behavior from git history (`eec1fa6^`) for `src/embedder.ts`, `src/retriever.ts`, and `src/store.ts` and mapped provider contract details to Rust.
  - completed Rust external embedding provider path (`openai-compatible`) with real HTTP request/response handling and env-placeholder secret resolution; fixed request-body bug so embedding JSON is now sent to provider.
  - completed Rust candidate retrieval path to use LanceDB vector search + FTS candidate generation (`fetch_recall_seeds`, `query_vector_candidates`, `query_fts_candidates`) with backend-owned ACL filtering.
  - completed Rust provider-aware rerank path for configured cross-encoder mode (Jina/SiliconFlow/Voyage/Pinecone/vLLM request/response adapters) with lightweight fallback.
  - kept `/v1` DTO contract stable (no internal vector/BM25/rerank component fields added to response rows).
  - added focused integration tests proving progress beyond hashing/lightweight-only defaults:
    - `openai_compatible_embedding_provider_is_used_for_recall`
    - `openai_compatible_embedding_provider_failure_returns_upstream_error`
    - `cross_encoder_rerank_provider_can_reorder_candidates`
- Evidence commands:
  - `cargo fmt --manifest-path backend/Cargo.toml`
  - `cargo check --manifest-path backend/Cargo.toml`
  - `cargo test --manifest-path backend/Cargo.toml`
  - `cargo test --manifest-path backend/Cargo.toml openai_compatible_embedding_provider_is_used_for_recall -- --exact --nocapture`
  - `cargo test --manifest-path backend/Cargo.toml cross_encoder_rerank_provider_can_reorder_candidates -- --exact --nocapture`
- Evidence results:
  - `cargo fmt`: pass
  - `cargo check`: pass (non-blocking warning remains: `ErrorCode::RateLimited` is never constructed)
  - `cargo test`: pass (21 tests passed, 0 failed)
  - focused embedding-provider integration test: pass
  - focused cross-encoder rerank test: pass
- Issues/blockers:
  - initial openai-compatible embedding test failed with `UPSTREAM_EMBEDDING_ERROR` because Rust request path did not send JSON body.
- Resolutions:
  - updated `OpenAiCompatibleEmbedder::embed_many()` to send payload via `request.json(&payload)`.
  - re-ran full verification and focused tests after fix.
- Residual risks:
  - advanced TS-only retrieval features remain unported in Rust (embedding LRU cache + key rotation/failover, query expansion/noise filtering/access-reinforcement telemetry, retrieval trace diagnostics).
  - schema-migration path for pre-existing Lance tables without `vector` column is not implemented in this batch; current behavior assumes new table creation for vector-enabled schema.
- Checkpoint confirmed: yes

### 2026-03-16 continuation (schema/index hardening + retrieval parity closeout batch)
- Phase: 2-4
- Batch date: 2026-03-16
- Completed tasks:
  - implemented explicit legacy LanceDB schema compatibility for pre-vector tables in `backend/src/state.rs`:
    - detect missing `vector` column via runtime schema inspection
    - create auditable backup table (`memories_v1_legacy_backup_<ts>`)
    - rebuild `memories_v1` with vector-capable schema
    - preserve legacy rows and attempt vector backfill; if provider backfill fails, keep rows with null vectors instead of failing migration
    - enforce explicit vector-dimension compatibility error when table vector dimension drifts from configured embedding dimension
  - implemented explicit vector-index lifecycle management in Rust backend:
    - preserved text FTS lifecycle (`ensure_text_fts_index`)
    - added `ensure_vector_index` with explicit ANN index creation on `vector` (`IVF_FLAT`) once rows exist
    - removed silent best-effort behavior on core index ensures by propagating failures
  - ported high-value TS retrieval parity features in Rust generic recall path:
    - query expansion (`expand_query_terms`) for lexical candidate retrieval/ranking
    - noise filtering (`is_noise_memory_text`) before final response ranking
    - backend-internal retrieval diagnostics hook (`retrieval.diagnostics`) via internal logs only (no `/v1` DTO drift)
  - implemented safe embedding reuse and credential failover seam for openai-compatible embedding provider:
    - in-process embedding cache with TTL/LRU-like bounded eviction
    - multi-key parsing (`api_key` comma/semicolon/newline list)
    - round-robin key selection + retry failover on retryable upstream statuses
  - extended backend config for new safe controls:
    - `providers.embedding.cache_max_entries`
    - `providers.embedding.cache_ttl_ms`
    - `retrieval.query_expansion`
    - `retrieval.filter_noise`
    - `retrieval.diagnostics`
  - added focused tests proving the new gaps are concretely closed:
    - `legacy_table_without_vector_column_is_migrated_without_data_loss`
    - `lancedb_search_indices_are_explicitly_ensured`
    - `embedding_provider_cache_reuses_vectors_across_write_and_recall`
    - `embedding_provider_rotates_keys_and_fails_over_on_auth_error`
    - `query_expansion_and_noise_filtering_improve_generic_recall`
- Evidence commands:
  - `cargo fmt --manifest-path backend/Cargo.toml`
  - `cargo check --manifest-path backend/Cargo.toml`
  - `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
  - `cargo test --manifest-path backend/Cargo.toml`
- Evidence results:
  - `cargo fmt`: pass
  - `cargo check`: pass (non-blocking warning remains: `ErrorCode::RateLimited` is never constructed)
  - `cargo test --test phase2_contract_semantics`: pass (26 passed, 0 failed)
  - `cargo test`: pass (26 passed, 0 failed)
- Issues/blockers:
  - initial vector-index creation attempt using `Index::Auto` failed on small row counts (`Not enough rows to train PQ`).
  - fallback attempt with `train(false)` also failed (`Creating empty vector indices with train=False is not yet implemented`).
- Resolutions:
  - switched explicit vector index creation path to `Index::IvfFlat(Default::default())` with replace semantics.
  - re-ran full backend verification after index strategy correction.
- Residual risks:
  - retrieval diagnostics are currently log-based internal hooks only; no dedicated admin trace surface exists yet.
  - key rotation/failover seam is implemented for embedding provider path; rerank provider key rotation remains deferred.
  - advanced TS access-reinforcement telemetry parity remains partially deferred (Rust has ranking/recency/decay controls but not full TS telemetry snapshot surface).
- Checkpoint confirmed: yes

## Final Release Gate
- Scope constraints preserved.
- Quality/security gates passed.
- Remaining risks documented.
