---
description: Deferred P2 parity work for rust-rag-completion.
---

# Tasks: rust-rag-completion Phase 6

## Input
- /root/verify/memory-lancedb-pro-context-engine-split/README.md
- /root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/rust-rag-parity-gap-priority.md
- /root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/task-plans/phase-5-rust-rag-completion.md
- /root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/task-plans/4phases-checklist.md
- /root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/rust-rag-completion-contracts.md
- /root/verify/memory-lancedb-pro-context-engine-split/docs/rust-rag-completion/rust-rag-completion-scope-milestones.md

## Canonical architecture / Key constraints
- Keep Rust backend as the only runtime RAG authority.
- Preserve stable `/v1` DTO boundaries.
- Do not restore deleted local TS authority paths.
- P2 work must remain low-risk and mergeable; if a specific item becomes too invasive, record it explicitly instead of forcing parity.

## Format
- [ID] [P?] [Component] Description
- [P] means parallelizable.
- Valid components: Backend, Frontend, Agentic, Docs, Config, QA, Security, Infra.
- Every task must have a clear DoD.

## Phase 6: deferred P2 parity closeout
Goal: land the remaining worthwhile legacy-TS parity improvements that are not required for P0/P1 production readiness.

Definition of Done: each P2 item is either implemented with focused verification or explicitly deferred again with exact technical reason.

Tasks:
- [x] T601 [Backend] Port access-reinforcement-aware time decay semantics.
  - DoD: Rust retrieval scoring can use access metadata to extend effective half-life in a bounded, testable way comparable to historical TS behavior; focused tests cover reinforcement impact and bounds.
- [x] T602 [Backend] Add backend-level diversity/MMR parity.
  - DoD: backend retrieval finalization includes a bounded diversity pass that reduces near-duplicate top-k domination without destabilizing deterministic ranking semantics; focused tests cover duplicate-heavy candidate sets.
- [x] T603 [Backend] Add provider-specific embedding tuning knobs.
  - DoD: Rust embedding provider config and requests can carry the safe subset of historical TS tuning knobs (`taskQuery`, `taskPassage`, `normalized`) where provider-compatible, with validation/tests and no DTO drift.
- [x] T604 [P] [QA] Add regression coverage for P2 behavior.
  - DoD: tests cover reinforcement decay, MMR/diversity behavior, and provider-specific embedding knob request shaping.
- [x] T605 [Docs] Update rust-rag docs/checklists with exact evidence and any newly deferred leftovers.
  - DoD: checklist and parity-gap docs clearly state what Phase 6 landed and what, if anything, remains deferred after P2.

Checkpoint: after Phase 6, parity discussion should move from implementation gaps to final acceptance/merge judgment.

## Dependencies & Execution Order
- T601 and T602 affect scoring semantics and should be reviewed carefully.
- T603 should avoid provider-specific overreach; keep it constrained to documented safe knobs.
- T604/T605 close the phase after implementation verification.

## Implementation Notes (2026-03-16)
- Added bounded access-reinforcement time decay in `backend/src/state.rs`:
  - persisted access metadata columns (`access_count`, `last_accessed_at`);
  - effective half-life extension with `retrieval.reinforcement_factor` + `retrieval.max_half_life_multiplier`;
  - best-effort access metadata updates after recall.
- Added backend deterministic diversity pass in `backend/src/state.rs`:
  - `retrieval.mmr_diversity` + `retrieval.mmr_similarity_threshold`;
  - `apply_mmr_diversity()` before final top-k truncation.
- Added provider-specific embedding knobs in `backend/src/config.rs` + `backend/src/state.rs`:
  - `providers.embedding.task_query` (`taskQuery` alias),
  - `providers.embedding.task_passage` (`taskPassage` alias),
  - `providers.embedding.normalized`,
  - conservative request-field emission only for compatible OpenAI-compatible assumptions.
- Added focused contract tests in `backend/tests/phase2_contract_semantics.rs`:
  - `access_reinforcement_extends_time_decay_for_old_memories`
  - `access_reinforcement_respects_max_half_life_multiplier_bound`
  - `mmr_diversity_reduces_duplicate_topk_deterministically`
  - `embedding_tuning_knobs_are_sent_for_compatible_provider_assumptions`
  - `embedding_tuning_knobs_are_omitted_when_provider_contract_is_not_compatible`

## Verification Snapshot
- `cargo fmt --manifest-path backend/Cargo.toml`: pass
- `cargo check --manifest-path backend/Cargo.toml`: pass (non-blocking pre-existing warning remains: `ErrorCode::RateLimited` is never constructed)
- `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`: pass (36 passed, 0 failed)
- `cargo test --manifest-path backend/Cargo.toml`: pass (36 passed, 0 failed)
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/rust-rag-completion`: pass (`[OK] placeholder scan clean`)
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/rust-rag-completion README.md`: pass (`[OK] post-refactor text scan passed`)
- Deferred after Phase 6 P2 batch: none.

## Reviewer Blocker Follow-up (2026-03-16)
- Scope: fix two reviewer-confirmed regressions from the Phase 6 P2 batch only.

### Blocker 1: embedding cache write+recall reuse
- Failing behavior: `embedding_provider_cache_reuses_vectors_across_write_and_recall` observed 2 provider calls instead of 1 for identical write+recall text.
- Root cause: cache reuse must be keyed by effective embedding request shape; any query/passage key drift reintroduces duplicate upstream calls.
- Fix kept in backend:
  - cache key remains derived from model + dimensions + effective tuning markers (`task`, `normalized`) + text digest, rather than raw call site identity;
  - cache remains shared across write and recall paths via `OpenAiCompatibleEmbedder` shared state and request-level dedup in `embed_many_with_purpose`.
- Evidence:
  - `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics embedding_provider_cache_reuses_vectors_across_write_and_recall -- --exact --nocapture`: pass.

### Blocker 2: deterministic MMR top-k ordering
- Failing behavior: `mmr_diversity_reduces_duplicate_topk_deterministically` could produce duplicate choice/order drift across repeated recalls.
- Root cause: MMR requires deterministic pre-diversity ordering and deterministic defer/append behavior under near-duplicate candidates.
- Fix kept in backend:
  - pre-MMR ranking uses deterministic tie-breakers (`score desc`, `updated_at desc`, `id asc`);
  - `apply_mmr_diversity` preserves deterministic first-pass selection and appends deferred rows in deterministic encounter order.
- Evidence:
  - `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics mmr_diversity_reduces_duplicate_topk_deterministically -- --exact --nocapture`: pass.

### Follow-up Verification
- `cargo fmt --manifest-path backend/Cargo.toml`: pass
- `cargo check --manifest-path backend/Cargo.toml`: pass
- `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics embedding_provider_cache_reuses_vectors_across_write_and_recall -- --exact --nocapture`: pass
- `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics mmr_diversity_reduces_duplicate_topk_deterministically -- --exact --nocapture`: pass
- `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`: pass (36 passed, 0 failed)
- `cargo test --manifest-path backend/Cargo.toml`: pass (36 passed, 0 failed)
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/rust-rag-completion`: pass
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/rust-rag-completion README.md`: pass
- Deferred after blocker fix: none.
