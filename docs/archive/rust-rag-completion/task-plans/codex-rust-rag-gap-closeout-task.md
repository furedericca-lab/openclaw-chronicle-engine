Continue from the latest verified Rust RAG port run and close the highest-value remaining gaps toward old TS parity.

Repo: /root/verify/memory-lancedb-pro-context-engine-split
Branch: dev/context-engine-split
Previous supervised run: 20260316T031529Z-memory-lancedb-pro-context-engine-split-write

Read first:
- docs/rust-rag-completion/rust-rag-completion-contracts.md
- docs/rust-rag-completion/rust-rag-completion-scope-milestones.md
- docs/rust-rag-completion/task-plans/4phases-checklist.md
- previous run artifacts from 20260316T031529Z-memory-lancedb-pro-context-engine-split-write

Current verified state:
- Rust backend now has a real provider-based RAG main path:
  - OpenAI-compatible embedding provider integration
  - LanceDB vector-search candidate retrieval
  - FTS candidate retrieval
  - provider-aware cross-encoder rerank path
- Stable `/v1` DTO boundary remains preserved.

Remaining gaps to close in priority order:
1. Schema migration / compatibility for pre-existing LanceDB tables that do not yet contain the new `vector` column.
2. Explicit vector-index lifecycle management (do not rely on implicit/default behavior only).
3. Advanced TS-parity retrieval features still missing in Rust:
   - embedding cache / batch reuse where safe
   - key rotation / failover seam for provider credentials when applicable
   - query expansion and noise filtering improvements
   - retrieval diagnostics / traceability hooks that do not leak into stable `/v1` DTOs
4. Any missing tests needed to prove the above are real and not just configuration seams.

Mission:
- Implement the deepest safe portion of the remaining gaps without regressing the verified provider-based RAG path.
- Prioritize production-safety gaps first (schema migration + index lifecycle) before feature polish.
- Keep Rust as the only runtime authority; do not restore any deleted TS authority path.
- Keep `/v1` response DTOs free of raw vector/BM25/rerank breakdown internals.

Concrete asks:
A. Schema migration / compatibility
- Detect and handle existing Lance tables that were created before the `vector` column existed.
- Land a credible migration/backfill strategy or a clearly enforced compatibility path that prevents silent breakage.
- Add tests or validation coverage proving old-table compatibility behavior.

B. Vector index lifecycle
- Add explicit vector index creation/ensure logic if LanceDB requires it or benefits from it for stable production behavior.
- Keep FTS index lifecycle intact.
- Add at least one verification path or test proving index setup behavior is not best-effort handwaving.

C. Retrieval-quality parity improvements
- Port the most valuable old TS retrieval-quality behaviors still missing, especially query expansion/noise filtering and safe caching/reuse where feasible.
- Add focused tests that prove the behavior change materially affects retrieval quality or robustness.
- If key rotation/failover cannot be fully implemented safely, leave a real seam plus a documented deferred note instead of pretending it is done.

D. Diagnostics
- Add backend-internal retrieval diagnostics/traceability hooks or test-visible instrumentation only if they do not pollute stable data-plane DTOs.
- Prefer debug/admin-internal surfaces or test-only assertions over contract drift.

Required verification before stopping:
- cargo fmt --manifest-path backend/Cargo.toml
- cargo check --manifest-path backend/Cargo.toml
- cargo test --manifest-path backend/Cargo.toml
- add focused tests for any newly closed gap(s)
- update docs/rust-rag-completion/task-plans/4phases-checklist.md with commands, results, and residual risks

Deliverable expectation:
- exact files changed
- which remaining gaps were actually closed
- which ones remain deferred
- exact verification results
