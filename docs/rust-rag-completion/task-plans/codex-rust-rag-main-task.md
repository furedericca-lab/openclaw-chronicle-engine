Implement the real Rust-owned RAG pipeline for this repo in the active worktree.

Repo: /root/verify/memory-lancedb-pro-context-engine-split
Branch: dev/context-engine-split

Primary docs to follow:
- docs/rust-rag-completion/rust-rag-completion-contracts.md
- docs/rust-rag-completion/rust-rag-completion-scope-milestones.md
- docs/rust-rag-completion/task-plans/4phases-checklist.md

Current backend status:
- `backend/` compiles and current tests pass.
- `backend/src/state.rs::recall_generic()` is still placeholder logic.
- The deleted local TypeScript RAG authority must NOT be restored; Rust must now own the retrieval path.

Authoritative reference material to inspect before changing code:
- src/embedder.ts
- src/retriever.ts
- src/store.ts
- src/auto-recall-final-selection.ts
- src/reflection-recall.ts
- test/benchmark-fixtures.json
- docs/archive/2026-03-15-architecture-reset/remote-memory-backend/remote-memory-backend-contracts.md
- docs/archive/2026-03-15-architecture-reset/remote-memory-backend/technical-documentation.md

Mission:
1. Replace placeholder generic recall with a real Rust RAG pipeline.
2. Keep backend ownership of principal/scope/ACL/ranking decisions.
3. Keep stable `/v1` DTOs free of raw vector/BM25/rerank diagnostic breakdown fields.
4. Add meaningful verification proving generic recall is no longer placeholder-only.
5. Update `docs/rust-rag-completion/task-plans/4phases-checklist.md` with evidence and residual risks.

Expected implementation areas:
- backend/src/config.rs
- backend/src/state.rs
- backend/src/models.rs
- backend/src/error.rs
- backend/tests/*.rs as needed
- docs/rust-rag-completion/* where implementation evidence needs updates

Hard constraints:
- Do not reintroduce the deleted local TS RAG authority.
- Do not break existing route shapes unless strictly necessary; if unavoidable, update docs in the same batch.
- Prefer backend-owned provider config and runtime wiring in Rust.
- Keep the repo compiling at every step.
- Use deterministic ranking/output ordering where practical.
- If a full feature cannot land safely, leave an explicit real seam and documented remaining gap instead of a disguised placeholder.

Required verification before stopping:
- cargo fmt --manifest-path backend/Cargo.toml
- cargo check --manifest-path backend/Cargo.toml
- cargo test --manifest-path backend/Cargo.toml
- add/run at least one focused test proving non-placeholder generic recall behavior
- update docs/rust-rag-completion/task-plans/4phases-checklist.md with commands/results

Deliverable expectation:
- changed files summary
- what part of the Rust RAG chain is now real
- what remains deferred, if anything
- exact verification commands and outcomes
