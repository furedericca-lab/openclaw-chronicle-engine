# README Refresh Contract

## Context

The repository has evolved materially beyond the current `README.md` / `README_CN.md` wording.
Current reality now includes:
- clear local LanceDB mode vs remote backend mode split
- `src/context/*` prompt orchestration seams
- remote shell adapter/client paths
- optional Rust backend under `backend/`
- refined principal-identity contract for remote mode
- mode-aware config validation (`embedding` required in local mode, optional in remote mode)
- session strategy / reflection / self-improvement / markdown mirror capabilities

The top-level READMEs should be refactored to match the actual code and current contract state.

## Goals

1. Rewrite `README.md` and `README_CN.md` so they reflect the repository as it exists now.
2. Make installation/configuration guidance accurate for both:
   - local LanceDB mode
   - remote backend mode
3. Make the architecture section clearer and shorter than the current sprawling text where possible.
4. Preserve useful user-facing content (installation, config, capabilities, troubleshooting), but remove stale or misleading wording.
5. Keep English README and Chinese README semantically aligned, not necessarily word-for-word.

## Non-goals

- No code changes unless absolutely required for doc correctness.
- No release/version bump.
- No new product claims that are not already present in code/docs.

## Required content updates

### A. Positioning / overview
- Explain that the plugin now supports two runtime authority modes:
  - local LanceDB authority (existing/default path)
  - remote backend authority via `remoteBackend.*`
- Make it explicit that remote mode keeps prompt orchestration local while delegating memory authority to HTTP backend endpoints.
- Do not overclaim maturity beyond what code/docs support; if remote backend is MVP/advanced mode, say so clearly.

### B. Architecture
- Update the architecture section so it accurately describes:
  - `index.ts` as entrypoint and mode switch
  - `src/context/*` as local prompt orchestration
  - `src/backend-client/*` / `src/backend-tools.ts` as remote shell adapter/tooling
  - local LanceDB path (`store.ts`, `embedder.ts`, `retriever.ts`, `tools.ts`)
  - optional Rust backend under `backend/`
- The diagram should be clearer than the current version and should not imply a shipped standalone ContextEngine plugin.

### C. Configuration guidance
- Local mode:
  - `embedding` required
  - `dbPath` relevant
- Remote mode:
  - `remoteBackend.enabled/baseURL/authToken` required for authority switch
  - local embedding config optional / unused in remote mode
- Mention the final principal contract in practical terms:
  - remote mode requires real runtime principal identity for data-plane calls
  - recall-style reads skip if identity unavailable; writes/enqueue fail closed
- Keep this wording user-facing; do not drown README in internal review language.

### D. Feature/status presentation
- Keep the current major capabilities, but reorganize for readability.
- Ensure these are covered accurately:
  - hybrid retrieval
  - rerank providers
  - multi-scope isolation
  - auto-capture / auto-recall
  - session strategy
  - memoryReflection
  - selfImprovement
  - mdMirror
  - CLI / management tools
  - remote backend mode
- Clarify which features are local-mode only vs remote-mode supported where relevant.

### E. Installation and examples
- Keep practical install instructions.
- Add a compact “choose your mode” section with two minimal config examples:
  - local mode minimal config
  - remote mode minimal config
- Ensure examples do not contradict current schema or parse-time behavior.

### F. Cleanup / stale wording
- Remove or rewrite statements that are now misleading due to the refactor/contract closeout.
- Reduce duplicated explanation where possible.
- Keep contributor/community sections only if still appropriate, but avoid letting them dominate the README.

## Target files

- `README.md`
- `README_CN.md`

## Writing requirements

- `README.md`: concise, accurate technical English.
- `README_CN.md`: natural Simplified Chinese.
- Keep headings and structure aligned enough that users can switch between languages.
- Prefer shorter sections + clearer mode split over one giant wall of text.

## Verification

Minimum verification after edits:
- `git diff --check`
- manual inspection that both READMEs mention:
  - local vs remote mode
  - mode-aware embedding rule
  - current architecture split

## Deliverable

Return:
- status
- files changed
- summary of README structure changes
- any wording caveat if something could not be expressed cleanly without deeper code/docs changes
