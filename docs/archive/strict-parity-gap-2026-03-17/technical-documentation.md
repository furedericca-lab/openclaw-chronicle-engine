# strict-parity-gap-2026-03-17 Technical Documentation

## Canonical architecture for strict parity work

The strict-parity scope does not change the supported runtime split:

1. `backend/src/*` remains the memory authority.
2. `index.ts`, `src/backend-client/*`, and `src/backend-tools.ts` remain the adapter layer.
3. `src/context/*` remains the prompt-time orchestration layer.

The strict-parity work adds one stronger requirement:

- authority-layer retrieval behavior and authority-layer traceability must be owned and auditable on the backend side, not reconstructed ad hoc by local TS helper code.
- acceptable parity may be delivered through Rust-native or remote-native mechanisms rather than literal re-creation of old TS-local abstractions.

## Key constraints and non-goals

- no local-authority runtime fallback may return;
- stable `/v1` DTOs must not grow internal trace payloads by accident;
- admin/debug surfaces, if added, must be explicit and isolated;
- prompt rendering and session-local gating may stay local, but backend-vs-local ownership must be explicit per seam.

## Interfaces between components

### Backend retrieval authority

Current implemented interfaces:

- storage and mutation: `LanceMemoryRepo::{store, update, delete, list, stats}`
- recall: `LanceMemoryRepo::{recall_generic, recall_reflection}`
- ranking core: `GenericRecallEngine::rank_candidates()`
- observability today: structured internal diagnostics emitted through `emit_internal_diagnostic()`

Strict-parity requirement:

- backend traceability must describe ranking stages, fallback path selection, and result-finalization decisions in a way that is inspectable beyond transient stderr logs.
- the exact shape may differ from the historical TS trace objects if the Rust replacement provides equivalent debugging value.

### Adapter layer

Current interfaces:

- `src/backend-client/client.ts` handles transport and idempotent write behavior.
- `src/backend-tools.ts` registers remote-backed tools only.
- `index.ts` wires hooks and local planners.

Strict-parity requirement:

- adapter must not become the hidden owner of retrieval diagnostics semantics;
- if backend emits trace/debug identifiers, adapter responsibilities must remain transport-only unless explicitly documented.

### Local orchestration layer

Current interfaces:

- `src/context/auto-recall-orchestrator.ts`
- `src/context/reflection-prompt-planner.ts`
- `src/context/session-exposure-state.ts`
- `src/context/prompt-block-renderer.ts`

Related retained helper modules:

- `src/recall-engine.ts`
- `src/auto-recall-final-selection.ts`
- `src/reflection-recall.ts`
- `src/final-topk-setwise-selection.ts`

Strict-parity requirement:

- each retained helper must be classified as either:
  - prompt-local orchestration/presentation logic, or
  - backend-parity debt that should move to Rust.

Phase 1 frozen classification:

- prompt-local / acceptable local runtime helpers:
  - `src/context/auto-recall-orchestrator.ts`
  - `src/context/reflection-prompt-planner.ts`
  - `src/context/session-exposure-state.ts`
  - `src/context/prompt-block-renderer.ts`
  - `src/recall-engine.ts`
  - `src/adaptive-retrieval.ts`
- prompt-local final-selection helpers that remain acceptable because they shape prompt injection only after backend recall has completed:
  - `src/auto-recall-final-selection.ts`
  - `src/final-topk-setwise-selection.ts`
- retained local test/reference helpers, not active production debt by default:
  - `src/reflection-recall.ts`
  - `src/reflection-recall-final-selection.ts`

## Operational behavior

Startup/runtime mode remains unchanged:

- `remoteBackend.enabled=true` is required;
- all write/list/stats/reflection enqueue paths still depend on runtime principal identity;
- fail-open vs fail-closed route behavior remains as documented in current runtime docs.

Strict-parity work added:

- explicit debug retrieval-trace inspection routes:
  - `POST /v1/debug/recall/generic`
  - `POST /v1/debug/recall/reflection`
- stable backend-owned trace payloads returning query summary, ordered stages, fallback reasons, and final row ids;
- fixture-driven parity verification commands in backend/plugin test suites.

## Observability and error handling

Current state:

- backend emits structured JSON diagnostic events for fallback paths, rerank attempts/fallback, ranking summaries, and access-update failures;
- events are internal-only and do not alter `/v1` response schemas.

Implemented strict-parity improvement:

- backend now exposes a debug-scoped trace model that answers:
  - which stages ran;
  - which fallback paths were used;
  - how many candidates survived each stage;
  - what the final row id set was;
  - whether access metadata updates succeeded.
- ordinary `/v1/recall/*` DTO rows still omit trace/diagnostic fields.

Accepted implementation for this scope:

- debug-only endpoints exposing structured traces under the same runtime auth + principal boundary as the rest of the data plane.

Rejected implementations:

- leaking trace internals into ordinary recall DTOs;
- scattered ad hoc `eprintln!` messages without a stable schema.
- forcing a historical TS telemetry object model into Rust when a narrower Rust-native structure provides equivalent inspection value.

## Security model and hardening notes

- trace/admin/debug surfaces must respect the same authority boundaries as the rest of the backend or be admin-token-only by explicit design;
- sensitive inputs must be redacted or bounded before trace persistence/exposure;
- no trace surface should allow reconstruction of another principal's memory corpus without explicit operator/admin authorization.

## Test strategy mapping

Current baseline:

- `npm test -- --runInBand`
- `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`

Strict-parity additions now map to:

- backend trace/diagnostic tests:
  - stage/fallback/result-count assertions
  - DTO non-leakage assertions
  - authorization checks for debug trace routes
- ownership-boundary tests:
  - prove `setwise-v2` remains prompt-local only
  - prove backend output remains authoritative while local final-selection code only trims/injects already-returned rows
- representative scenario fixtures:
  - duplicate-heavy corpora
  - stale-but-reinforced memories
  - rerank fallback cases
  - reflection recall grouping/selection stability
