# strict-parity-gap-2026-03-17 Implementation Research Notes

## Problem statement and current baseline

Current canonical runtime architecture is already remote-authority:

- `backend/src/*` owns persistence, retrieval, ranking, scope derivation, ACL, reflection recall, and reflection job execution.
- `index.ts`, `src/backend-client/*`, and `src/backend-tools.ts` form the adapter layer.
- `src/context/*` keeps prompt-time orchestration and session-local state only.

Relevant current code paths:

- backend retrieval/ranking core: `backend/src/state.rs`
- backend config surface: `backend/src/config.rs`
- backend contract tests: `backend/tests/phase2_contract_semantics.rs`
- retained local prompt-time orchestration: `src/context/auto-recall-orchestrator.ts`, `src/context/reflection-prompt-planner.ts`, `src/context/session-exposure-state.ts`, `src/context/prompt-block-renderer.ts`
- retained local ranking helpers used by orchestration/tests: `src/recall-engine.ts`, `src/auto-recall-final-selection.ts`, `src/reflection-recall.ts`, `src/final-topk-setwise-selection.ts`

Historical deletion/cleanup baseline:

- local-authority runtime modules were intentionally removed: `src/store.ts`, `src/retriever.ts`, `src/embedder.ts`, `src/tools.ts`, `src/migrate.ts`, `src/scopes.ts`, `src/access-tracker.ts`, `cli.ts`
- evidence: `docs/archive/remote-authority-reset/remote-authority-reset-scope-milestones.md`, `docs/archive/remote-authority-reset/phase-4-closeout-release-notes.md`

Current Rust retrieval capability baseline from source:

- hybrid candidate generation: vector + FTS seed collection in `backend/src/state.rs`
- score merge, rerank, recency, importance, length normalization, time decay, and optional MMR in `GenericRecallEngine::rank_candidates()`
- access metadata persistence and post-recall updates in `record_recall_access_metadata()`
- provider-backed embedding chunk recovery and rerank failover in `OpenAiCompatibleEmbedder` / `RerankProviderClient`

Strict parity rule for this scope:

- parity is judged at the capability/behavior level;
- historical TS behavior may be satisfied by a Rust-native or remote-native replacement when the old TS-local form is no longer architecturally appropriate;
- old TS-local implementation shapes must not be treated as mandatory if the current remote-authority design already provides an equivalent or superior supported behavior.

## Frozen retained TS helper ownership matrix

This matrix is the Phase 1 freeze for retained helper ownership.

| Path | Current role | Classification | Freeze rationale |
| --- | --- | --- | --- |
| `src/context/auto-recall-orchestrator.ts` | Decides if/when generic recall rows should be injected into prompt context; consumes backend `/v1/recall/generic` results; applies local post-filtering and optional setwise final selection before rendering | Acceptable local orchestration | This file does not own storage, scope, ACL, or raw retrieval authority. Its remaining logic is prompt-time injection policy and output shaping. |
| `src/context/reflection-prompt-planner.ts` | Decides if/when reflection recall and error reminder blocks are prepended; consumes backend `/v1/recall/reflection` results | Acceptable local orchestration | It is a hook-facing planner for prompt assembly, not a backend substitute. |
| `src/context/session-exposure-state.ts` | Session-local dedupe/history state for recall and reflection error signals | Acceptable local orchestration | Session injection suppression and prompt exposure state are inherently runtime-local. |
| `src/context/prompt-block-renderer.ts` | Renders `<relevant-memories>`, `<inherited-rules>`, and `<error-detected>` blocks | Acceptable local orchestration | Pure presentation/rendering concern. |
| `src/recall-engine.ts` | Shared local helper for skip gates, session-level recall dedupe, max-age filtering, and normalized-key recency capping | Acceptable local orchestration with audit watchlist | Current usage is prompt-time gating/state management. It should remain local unless future evidence shows that one of these policies must become backend-authoritative. |
| `src/auto-recall-final-selection.ts` | Optional local final selector for auto-recall rows after backend recall, used when `selectionMode === "setwise-v2"` | Backend-parity debt candidate | This is the strongest retained candidate for migration because it still owns a non-trivial final ranking/diversity pass over backend rows rather than only rendering/gating. |
| `src/final-topk-setwise-selection.ts` | Generic setwise shortlist/final top-k utility used by local final-selection modules | Backend-parity debt candidate when used for authority-adjacent ranking | The module itself is generic, but its current production relevance depends on `src/auto-recall-final-selection.ts`. If auto-recall final selection moves to Rust, this file likely becomes test-only or removable. |
| `src/reflection-recall.ts` | Local ranking/aggregation over reflection entries, currently referenced by tests rather than the active remote reflection recall path | Acceptable retained test/reference helper for now | This file is not on the active remote production path. It should not be counted as production backend debt unless runtime code starts consuming it again. |
| `src/reflection-recall-final-selection.ts` | Final selection helper used by `src/reflection-recall.ts` | Acceptable retained test/reference helper for now | Same rationale as `src/reflection-recall.ts`; not active backend-authority debt today. |
| `src/adaptive-retrieval.ts` | Skip-gate heuristic for whether prompt-time retrieval should run at all | Acceptable local orchestration | This is a runtime prompt trigger heuristic, not memory authority logic. |

Phase 1 decision:

- treat `src/auto-recall-final-selection.ts` as the only retained helper requiring Phase 3 proof before final disposition;
- treat `src/final-topk-setwise-selection.ts` as derivative debt only if the auto-recall selector is later shown to own backend-authority semantics;
- treat the rest of the retained helpers as acceptable local orchestration or test/reference helpers under the remote-authority design.

## Frozen representative strict-parity scenario matrix

Phase 1 freezes the scenario set that later phases must use when deciding whether a gap is still open.

| Scenario | Why it matters | Current primary implementation path | Expected parity bar |
| --- | --- | --- | --- |
| Duplicate-heavy generic recall | Historical TS invested heavily in diversity and duplicate suppression | Backend ranking + optional local `setwise-v2` selector | Backend must provide acceptable diversity semantics; if local selector remains, docs/tests must justify it as acceptable prompt-local post-selection or migrate it |
| Old but frequently accessed memory | Confirms access-reinforcement-aware time decay parity | Rust backend time-decay plus access metadata update | Already satisfied by acceptable Rust-native backend behavior |
| Long input embedding overflow | Confirms safe recovery from provider context limits | Rust `OpenAiCompatibleEmbedder` chunk recovery | Already satisfied by acceptable Rust-native backend behavior |
| Rerank provider auth/rate-limit failure | Confirms multi-key failover and fallback behavior | Rust rerank client + lightweight fallback | Already satisfied by acceptable Rust-native backend behavior |
| Reflection recall grouping and inherited-rule injection | Distinguishes backend recall authority from local prompt assembly | Backend reflection recall + local planner/rendering | Acceptable split as long as ranking authority stays backend-owned and local layer remains assembly-only |
| Retrieval diagnostics / trace inspection | Historical TS had thicker debugging surfaces | Rust structured internal diagnostics only | Still open until an acceptable inspectable Rust-native trace surface is frozen and verified |

Phase 1 scenario decision:

- the parity target is not "old TS object model parity";
- the parity target is "equivalent operator/debug/runtime capability under remote Rust architecture";
- scenarios already closed by acceptable Rust-native behavior must not be reopened in later phases.

## Gap analysis with evidence

### Closed items that are not strict gaps anymore

1. **Rust generic recall is no longer placeholder-only.**
   Evidence:
   - `backend/src/state.rs` implements vector seed, FTS seed, hybrid merge, rerank, recency, length, importance, time-decay, and truncation.
   - `docs/archive/rust-rag-completion/rust-rag-parity-gap-priority.md` marks the main retrieval port as closed in Phase 5/6.

2. **Access-reinforcement-aware time decay is implemented.**
   Evidence:
   - `backend/src/state.rs`: `compute_effective_half_life_days()` and `record_recall_access_metadata()`
   - tests: `access_reinforcement_extends_time_decay_for_old_memories`, `access_reinforcement_respects_max_half_life_multiplier_bound`

3. **Backend-level diversity/MMR is implemented.**
   Evidence:
   - `backend/src/state.rs`: `apply_mmr_diversity()`
   - test: `mmr_diversity_reduces_duplicate_topk_deterministically`

4. **Old local-authority TS runtime chain is removed.**
   Evidence:
   - removed files are listed as complete in `docs/archive/remote-authority-reset/phase-4-closeout-release-notes.md`
   - current worktree does not contain those deleted modules.

### Strict remaining gaps under capability-equivalence review

1. **Structured diagnostics existed, but Phase 2 closed the inspectable traceability gap with explicit debug routes.**
   Evidence:
   - backend now exposes `POST /v1/debug/recall/generic` and `POST /v1/debug/recall/reflection`
   - trace payloads record query summary, ordered stages, fallback reason, counts, and final row ids while normal `/v1/recall/*` rows remain unchanged
   - backend tests cover route visibility, rerank fallback trace recording, principal-boundary enforcement, and DTO non-leakage in `backend/tests/phase2_contract_semantics.rs`
   Final disposition:
   - accepted as an architecture-appropriate Rust-native parity implementation; historical TS telemetry object shapes do not need to be recreated verbatim.

2. **Retained local TS ranking/orchestration helpers had ownership ambiguity, but Phase 3 resolved the only live seam as prompt-local.**
   Evidence:
   - `src/context/auto-recall-orchestrator.ts` imports `selectFinalAutoRecallResults` from `src/auto-recall-final-selection.ts`
   - `src/context/reflection-prompt-planner.ts` imports orchestration helpers from `src/recall-engine.ts`
   - `src/reflection-recall.ts` still performs local aggregation/ranking for retained reflection-entry flows
   Phase 3 closure evidence:
   - `test/memory-reflection.test.mjs` now proves `setwise-v2` requests backend candidates via the standard remote recall dependency, then applies duplicate suppression only while building prompt-local injected context
   - the backend request/response contract remains unchanged; local selector output is a prompt block, not a backend authority result
   Final disposition:
   - `src/auto-recall-final-selection.ts` and `src/final-topk-setwise-selection.ts` are acceptable prompt-local post-selection helpers under the remote-authority design.

3. **Parity verification previously over-indexed on contract behavior; the representative scenario matrix is now materially covered.**
   Evidence:
   - duplicate-heavy recall and diversity: backend MMR tests plus local `setwise-v2` tests
   - stale-but-reinforced memories: backend access-reinforcement tests
   - rerank fallback: backend provider failover tests plus debug trace fallback assertions
   - reflection grouping/selection: `test/memory-reflection.test.mjs`
   - trace visibility: new backend debug-route tests
   Final disposition:
   - acceptable parity coverage achieved for this scope; future regressions should extend these suites rather than reopen architecture ownership questions.

### Non-gaps that must not be misclassified

1. **`src/context/*` remaining in the repo is not itself stale residue.**
   Evidence:
   - `docs/runtime-architecture.md` explicitly defines `src/context/*` as the supported local prompt-time orchestration layer.

2. **`src/recall-engine.ts`, `src/auto-recall-final-selection.ts`, `src/reflection-recall.ts` are not proof of a surviving local-authority backend.**
   Evidence:
   - current imports show they support prompt-time orchestration and local selection logic, not persistence/scope authority.

## Architecture / implementation options and trade-offs

### Option 1: Keep current split and redefine strict parity downward to match existing docs

Pros:

- minimal work;
- no new backend observability scope.

Cons:

- evades the user's strict parity question;
- leaves "trace parity" undefined;
- future reviewers will keep reopening the same argument.

Decision:

- reject.

### Option 2: Close only observability/trace gaps, but leave TS final-selection ownership untouched

Pros:

- smallest backend change set;
- preserves current local orchestration boundaries.

Cons:

- still leaves ambiguity about whether backend truly owns the final retrieval semantics or only the initial ranking pipeline;
- strict parity remains partial.

Decision:

- possible fallback, but not the preferred plan.

### Option 3: Define strict parity as capability-equivalence closure with architecture-aware exceptions

Track A:

- close backend trace/diagnostics parity with a structured internal trace model and an explicit inspection mechanism.

Track B:

- audit and, where justified, migrate or explicitly freeze remaining TS-side final-selection logic so ownership boundaries are unambiguous.

Pros:

- gives a precise answer to what still differs from old TS;
- avoids reintroducing local authority while still shrinking leftover TS ownership;
- aligns code ownership, observability, and verification.

Cons:

- largest documentation and implementation scope;
- requires care to avoid moving clearly prompt-local concerns into the backend just for symmetry.

Decision:

- selected.

## Selected design and rationale

Selected strict-parity definition:

- backend must own all authority-layer retrieval semantics;
- local TS may retain prompt-time rendering/session gating and other prompt-local logic;
- any remaining TS-side ranking/final-selection logic must either:
  - move into Rust, or
  - be explicitly accepted as prompt-local capability that does not weaken backend authority;
- diagnostics parity requires a structured, inspectable retrieval trace story stronger than current log-event emission, but not necessarily a literal recreation of historical TS telemetry objects.

Implementation direction:

- phase 1 freezes the strict gap register and extracts representative historical scenarios;
- phase 2 targets backend diagnostics/trace parity with acceptable Rust-native observability;
- phase 3 resolves TS-vs-Rust ownership ambiguity for retained final-selection helpers and upgrades verification.

## Test and validation strategy

Documentation validation:

```bash
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/archive/strict-parity-gap-2026-03-17
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/archive/strict-parity-gap-2026-03-17 README.md
```

Implementation-time validation expected from this scope:

- backend:
  - `cargo fmt --manifest-path backend/Cargo.toml`
  - `cargo check --manifest-path backend/Cargo.toml`
  - `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
  - focused new trace/diagnostic tests if trace storage or admin/debug endpoints are added
- plugin:
  - `npm test -- --runInBand`
  - focused tests for orchestration ownership changes if TS final-selection logic moves

Strict-parity verification should additionally include:

- fixture-driven comparisons for duplicate-heavy recall, stale-vs-reinforced memories, fallback/retry behavior, and reflection recall ordering;
- assertions that internal trace surfaces expose stage decisions without leaking into stable `/v1` DTOs;
- evidence that any retained TS helper is either prompt-local only or deliberately migrated because it still encodes authority semantics.

## Risks, assumptions, unresolved questions

Risks:

- pushing all remaining ranking helpers into Rust may accidentally couple backend DTOs to prompt-format needs;
- adding a trace/admin/debug surface can create a misleading second API unless clearly isolated from ordinary runtime routes;
- historical TS parity may include behaviors that are no longer desirable under remote authority.

Assumptions:

- current backend contract tests are a safe foundation for deeper parity verification;
- archive docs plus retained helper modules provide enough evidence to reconstruct the meaningful historical TS bar;
- strict parity can be improved without reviving deleted local-authority files.

Unresolved questions:

- should the strict trace surface be persisted in SQLite, emitted only as structured test hooks, or exposed via explicit admin/debug endpoints, and which of those options is sufficient for acceptable Rust-native parity;
- which retained TS helpers are truly prompt-local and should remain, versus which are latent backend-parity debt.
