---
description: P0/P1 parity closeout tasks for rust-rag-completion.
---

# Tasks: rust-rag-completion Phase 5

## Input
- /root/verify/memory-lancedb-pro-context-engine-split/README.md
- /root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/rust-rag-parity-gap-priority.md
- /root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/rust-rag-completion-scope-milestones.md
- /root/verify/memory-lancedb-pro-context-engine-split/docs/archive/rust-rag-completion/rust-rag-completion-technical-documentation.md
- /root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/rust-rag-completion-contracts.md
- /root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/task-plans/4phases-checklist.md

## Canonical architecture / Key constraints
- Keep Rust backend as the only runtime RAG authority.
- Preserve stable `/v1` DTO boundaries; no internal diagnostics or score-breakdown fields may leak into runtime data-plane responses.
- Do not restore deleted local TS authority paths.
- Prioritize production resilience and auditable internal observability over parity-for-parity polish.

## Format
- [ID] [P?] [Component] Description
- [P] means parallelizable.
- Valid components: Backend, Frontend, Agentic, Docs, Config, QA, Security, Infra.
- Every task must have a clear DoD.

## Phase 5: P0/P1 parity closeout
Goal: close the remaining high-value Rust-vs-legacy-TS gaps that are worth landing before merge.

Definition of Done: P0 and P1 gaps from `rust-rag-parity-gap-priority.md` are implemented and verified, or explicitly blocked with exact evidence and no ambiguity.

Tasks:
- [x] T501 [Backend] Add provider-backed long-text embedding chunking / context-limit recovery.
  - DoD: `backend/src/state.rs` (and any extracted helper modules if needed) can recover from provider context-window overflow using safe chunking/aggregation semantics comparable to historical TS behavior; focused tests prove recovery and non-recovery edge cases.
- [x] T502 [Backend] Add rerank provider multi-key rotation and retry/failover.
  - DoD: rerank config accepts multi-key input with deterministic retry/failover on retryable upstream auth/rate-limit/service errors; focused tests cover success after failover and non-retryable behavior.
- [x] T503 [Backend] Replace ad-hoc retrieval diagnostic prints with structured internal diagnostics.
  - DoD: diagnostics remain internal-only (no `/v1` DTO drift), but logs/test-visible hooks include structured stage/fallback counts and reasons that are auditable and stable enough for future admin/debug surfaces.
- [x] T504 [P] [QA] Add/expand regression coverage for P0/P1 closeout behavior.
  - DoD: `backend/tests/phase2_contract_semantics.rs` or split test files cover chunking recovery, rerank key failover, and diagnostics invariants; full backend test suite passes.
- [x] T505 [Docs] Update active rust-rag docs/checklists with exact commands, outcomes, and explicit deferred P2 items.
  - DoD: `docs/rust-rag-completion/task-plans/4phases-checklist.md` and/or phase docs record what landed, how it was verified, and what remains deliberately deferred.

Checkpoint: after Phase 5, the remaining open items should be deliberate P2 deferrals rather than ambiguous parity gaps.

## Dependencies & Execution Order
- T501 and T502 are the primary implementation gates.
- T503 may proceed in parallel only if it does not destabilize retrieval semantics.
- T504/T505 close the phase after code behavior is verified.

## Execution Record (2026-03-16)

### Implemented
- P0 embedding parity:
  - added provider-backed context-limit recovery for long text in `OpenAiCompatibleEmbedder` (`backend/src/state.rs`):
    - detects provider context-limit failures;
    - applies smart chunking;
    - retries chunk embeddings with existing credential failover;
    - averages chunk vectors into a stable final embedding.
- P1 rerank resilience parity:
  - upgraded `RerankProviderClient` from single key to multi-key rotation with retry/failover on retryable upstream statuses (401/403/429/5xx class and transient transport errors), while preserving lightweight fallback semantics on terminal rerank failure.
- P1 diagnostics parity:
  - replaced ad-hoc retrieval-path diagnostic prints with structured internal JSON diagnostics events for:
    - seed fallback stages (`vector-search`, `fts-search`, `full-scan`);
    - rerank provider attempts/fallback;
    - retrieval ranking summary (seed/selected/noise-filtered/result counts).
  - `/v1` DTO boundaries remain unchanged.

### Focused test evidence
- Added/expanded tests in `backend/tests/phase2_contract_semantics.rs`:
  - `openai_compatible_embedding_context_limit_recovers_with_chunking`
  - `openai_compatible_embedding_context_limit_recovery_failure_returns_upstream_error`
  - `rerank_provider_rotates_keys_and_fails_over_on_auth_error`
  - `rerank_provider_does_not_rotate_on_non_retryable_error`
  - `retrieval_diagnostics_enabled_does_not_leak_internal_fields_to_v1_rows`

### Verification commands and outcomes
- `cargo fmt --manifest-path backend/Cargo.toml`: pass
- `cargo check --manifest-path backend/Cargo.toml`: pass (non-blocking pre-existing warning remains: `ErrorCode::RateLimited` is never constructed)
- `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`: pass (31 passed, 0 failed)
- `cargo test --manifest-path backend/Cargo.toml`: pass (31 passed, 0 failed)
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/rust-rag-completion`: pass (`[OK] placeholder scan clean`)
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/rust-rag-completion README.md`: pass (`[OK] post-refactor text scan passed`)

### Explicit deferred items (P2 only)
- access-reinforcement-aware time decay parity remains deferred.
- backend-level diversity/MMR parity remains deferred.
- provider-specific embedding tuning knobs (`taskQuery`, `taskPassage`, `normalized`) remain deferred.
