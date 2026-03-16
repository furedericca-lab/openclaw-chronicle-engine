Continue from the previous Rust RAG implementation batch and port the old TypeScript full RAG capabilities into the Rust backend.

Repo: /root/verify/memory-lancedb-pro-context-engine-split
Branch: dev/context-engine-split
Previous supervised run: 20260316T024345Z-memory-lancedb-pro-context-engine-split-write

Read first:
- docs/rust-rag-completion/rust-rag-completion-contracts.md
- docs/rust-rag-completion/rust-rag-completion-scope-milestones.md
- docs/rust-rag-completion/task-plans/4phases-checklist.md
- previous run artifacts from 20260316T024345Z-memory-lancedb-pro-context-engine-split-write

Current state to continue from:
- Rust generic recall is no longer placeholder-only.
- Current implementation is still lightweight / incomplete:
  - deterministic hashing embedder only
  - lightweight rerank only
  - in-memory candidate scoring over caller-scoped rows
- User explicitly wants the old TypeScript full RAG capability ported into Rust.

Mission for this continuation:
1. Recover and inspect the old TypeScript RAG implementation from current repo files and git history if files are no longer present in the worktree.
2. Port the real TS-side RAG authority into Rust as far as possible in this worktree:
   - real embedding provider integration (not hashing-only)
   - real candidate retrieval path rather than only full in-memory scan scoring
   - real rerank/provider path when configured
   - preserve backend-owned ACL / scope / DTO boundaries
3. Keep the stable `/v1` DTO contract unchanged unless absolutely required.
4. Add focused tests and docs evidence for the newly ported behavior.
5. Update `docs/rust-rag-completion/task-plans/4phases-checklist.md` with exact commands/results/residual risks.

Required source investigation targets:
- live repo files if present
- git history for removed TS modules such as:
  - src/embedder.ts
  - src/retriever.ts
  - src/store.ts
- remaining live references:
  - src/auto-recall-final-selection.ts
  - src/reflection-recall.ts
  - test/benchmark-fixtures.json
  - docs/archive/2026-03-15-architecture-reset/remote-memory-backend/*.md

Hard requirements:
- Do NOT reintroduce the deleted TS authority path.
- Rust must remain the only runtime authority.
- If external-provider integration cannot be completed fully in one batch, land the deepest safe portion with explicit seams and no disguised placeholder behavior.
- Keep the repo compiling and tested.

Verification target before stopping:
- cargo fmt --manifest-path backend/Cargo.toml
- cargo check --manifest-path backend/Cargo.toml
- cargo test --manifest-path backend/Cargo.toml
- add or update focused tests proving progress beyond hashing/lightweight-only behavior when possible

Deliverable expectation:
- exact files changed
- what portion of old TS full RAG was ported
- what still remains unported
- exact verification results
