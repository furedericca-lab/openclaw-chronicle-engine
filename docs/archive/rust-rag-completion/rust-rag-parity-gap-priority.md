# Rust RAG Parity Gap Priority

## Context
This document ranks the remaining post-mainline Rust-vs-legacy-TS RAG gaps on `dev/context-engine-split` after the Rust backend achieved provider-based generic recall, reflection recall, schema migration, explicit index lifecycle, query expansion, noise filtering, embedding cache, and embedding key failover.

## Status Update (2026-03-16, Phase 6)
- P0 and P1 items are already closed from Phase 5.
- All previously deferred P2 items are now implemented in Rust backend code and covered by focused contract tests.
- Remaining deferred parity work after this P2 batch: **none**.

## Priority Buckets

### P0 — close before claiming production-ready parity
1. **OpenAI-compatible embedding long-text chunking / context-limit recovery**
   - Why: legacy TS `src/embedder.ts` handled provider context-window overflow via `smartChunk()` and retry. Current Rust `OpenAiCompatibleEmbedder` does not show equivalent recovery logic, so long documents can still fail hard on provider-backed embedding paths.
   - Evidence:
     - Historical TS: `eec1fa6^:src/embedder.ts`
     - Current Rust: `backend/src/state.rs` (`OpenAiCompatibleEmbedder`)
   - Acceptance:
     - provider-backed long-text embedding no longer fails hard on context-length overflow when chunking can recover safely;
     - focused tests prove recovery behavior.

### P1 — high-value resilience / observability parity
2. **Rerank provider multi-key rotation and retry/failover**
   - Why: embedding provider already supports multi-key parsing + round-robin retry/failover, but rerank provider still uses a single key path. This is a resilience gap, not a correctness blocker.
   - Evidence:
     - Current Rust embedding: `backend/src/state.rs` (`api_keys`, `request_embeddings_with_failover`)
     - Current Rust rerank: `backend/src/state.rs` (`RerankProviderClient { api_key: Option<String> }`)
   - Acceptance:
     - rerank provider accepts multi-key input, retries/fails over on retryable auth/upstream errors, and keeps current fallback semantics.

3. **Structured internal retrieval diagnostics**
   - Why: current Rust diagnostics are log-print based (`eprintln!`) and much thinner than historical TS retrieval trace/telemetry. This should stay off `/v1` DTOs but become more structured and auditable internally.
   - Evidence:
     - Current Rust: `backend/src/state.rs` (`retrieval.diagnostics`, `eprintln!`)
     - Historical TS: `eec1fa6^:src/retriever.ts` (`RetrievalTrace`, `scoreHistory`, telemetry snapshots)
   - Acceptance:
     - diagnostics become structured/internal (for logs/tests/admin-only use), include stage/candidate/result counts and failure/fallback reasons, and remain absent from `/v1` runtime DTOs.

### P2 — useful parity polish, not required for current merge
4. **Access-reinforcement-aware time decay parity** (closed in Phase 6)
   - Implemented in `backend/src/state.rs` with bounded access metadata (`access_count`, `last_accessed_at`) and capped effective half-life extension.
   - Focused tests:
     - `access_reinforcement_extends_time_decay_for_old_memories`
     - `access_reinforcement_respects_max_half_life_multiplier_bound`

5. **Backend-level diversity/MMR parity** (closed in Phase 6)
   - Implemented in `backend/src/state.rs` as deterministic backend MMR pass (`apply_mmr_diversity`) before top-k truncation.
   - Focused test:
     - `mmr_diversity_reduces_duplicate_topk_deterministically`

6. **Provider-specific embedding tuning knobs** (closed in Phase 6)
   - Implemented in `backend/src/config.rs` + `backend/src/state.rs` for `taskQuery`/`taskPassage`/`normalized` with conservative provider-compatibility gating.
   - Focused tests:
     - `embedding_tuning_knobs_are_sent_for_compatible_provider_assumptions`
     - `embedding_tuning_knobs_are_omitted_when_provider_contract_is_not_compatible`

## Execution Decision
- Historical decision: Phase 5 intentionally closed only P0/P1 and deferred P2.
- Current status: Phase 6 closed all deferred P2 items with focused tests and no `/v1` DTO boundary drift.
