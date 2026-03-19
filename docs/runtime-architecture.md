# Runtime Architecture

## Canonical runtime split

`Chronicle Engine` now has one supported runtime architecture:

1. `backend/` is the memory authority.
   - Owns persistence, retrieval, ranking, scope derivation, ACL, behavioral-guidance recall data, and distill execution.
   - Owns provider-backed embedding and rerank behavior.
2. `index.ts`, `src/backend-client/*`, and `src/backend-tools.ts` are the OpenClaw adapter layer.
   - Own hook and tool registration.
   - Translate runtime context into backend requests.
   - Apply transport retry, auth header wiring, and route-level fail-open vs fail-closed behavior.
3. `src/context/*` is the local prompt-time orchestration layer.
   - Decides when to recall or inject.
   - Renders `<relevant-memories>`, `<behavioral-guidance>`, and `<error-detected>` blocks.
   - Keeps only session-local orchestration state.
   - Must not own backend-visible candidate filtering semantics such as category, reflection-kind, score-threshold, age-window, or per-key recall caps.

## Frozen ownership split

- `distill` is the only write path that derives new knowledge from session trajectories.
- `autoRecall` is the only prompt-time recall/injection orchestration surface.
- `autoRecall` has context and behavioral-guidance channels; neither is a separate generation pipeline.
- `governance` owns backlog, review, extraction, and promotion workflow tools/files.
- Governance tools are registered only under canonical `governance_*` names.
- `session-lessons` owns lesson, cause, fix, prevention, stable decision, and durable practice extraction.
- `governance-candidates` owns worth-promoting learnings, skill extraction candidates, and AGENTS/SOUL/TOOLS promotion candidates.
- `follow-up-focus` and `next-turn-guidance` are distill artifact subtypes, not a separate reflection persistence pipeline.

## Trigger model

- Ordered session transcript rows are appended on `agent_end`.
- Automatic generation happens only through cadence-driven distill via `distill.everyTurns`.
- Command lifecycle hooks no longer trigger non-distill generation jobs.

## Runtime invariants

- `remoteBackend.enabled=true` is required for supported runtime behavior.
- The plugin does not provide a supported local-authority fallback.
- Client-side tools do not own scope selection; backend data-plane routes remain authoritative.
- The plugin no longer ships self-improvement/reflection wrapper modules or legacy governance tool aliases.
- Recall/injection paths remain fail-open where appropriate.
- Write, update, delete, list, stats, and distill enqueue paths require runtime principal identity and fail clearly when that identity is missing.

## Naming boundary

- Canonical plugin/runtime language is `autoRecall`, `behavioral guidance`, `distill`, and `governance`.
- The backend wire/storage contract still uses `category=reflection` and `/v1/recall/reflection` for the behavioral-guidance recall lane in this scope.
- That backend naming is intentionally retained for contract and persisted-data stability; adapter/runtime internals map it to behavioral-guidance terminology instead of exposing new compatibility shims.

## Current source-of-truth files

- Runtime entrypoint and config validation: `index.ts`
- Plugin config schema: `openclaw.plugin.json`
- Backend transport/types: `src/backend-client/*`
- Tool bridge: `src/backend-tools.ts`
- Prompt orchestration: `src/context/*`
- Rust backend implementation: `backend/src/*`
- Deployment examples: `deploy/README.md`, `deploy/backend.toml.example`, `deploy/docker-compose.yml`

## Historical material

Older transition documents are preserved under `docs/archive/`. They remain useful for reconstruction and audit, but they are no longer canonical references for the current repository state.
