# Runtime Architecture

## Canonical runtime split

`Chronicle Engine` now has one supported runtime architecture:

1. `backend/` is the memory authority.
   - Owns persistence, retrieval, ranking, scope derivation, ACL, reflection recall, and reflection job execution.
   - Owns provider-backed embedding and rerank behavior.
2. `index.ts`, `src/backend-client/*`, and `src/backend-tools.ts` are the OpenClaw adapter layer.
   - Own hook and tool registration.
   - Translate runtime context into backend requests.
   - Apply transport retry, auth header wiring, and route-level fail-open vs fail-closed behavior.
3. `src/context/*` is the local prompt-time orchestration layer.
   - Decides when to recall or inject.
   - Renders `<relevant-memories>`, `<inherited-rules>`, and `<error-detected>` blocks.
   - Keeps only session-local orchestration state.
   - Must not own backend-visible candidate filtering semantics such as category, reflection-kind, score-threshold, age-window, or per-key recall caps.

## Runtime invariants

- `remoteBackend.enabled=true` is required for supported runtime behavior.
- The plugin does not provide a supported local-authority fallback.
- Client-side tools do not own scope selection; backend data-plane routes remain authoritative.
- Recall/injection paths remain fail-open where appropriate.
- Write, update, delete, list, stats, and reflection enqueue paths require runtime principal identity and fail clearly when that identity is missing.

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
