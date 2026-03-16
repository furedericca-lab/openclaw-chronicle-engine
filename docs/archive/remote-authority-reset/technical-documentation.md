---
description: Canonical technical architecture for remote-authority-reset.
---

# remote-authority-reset Technical Documentation

## Canonical Architecture

Target runtime architecture:

1. **Rust remote backend**
   - lives under `backend/`;
   - is the only authority for:
     - memory and reflection persistence,
     - retrieval and ranking,
     - embedding/rerank/reflection provider execution,
     - ACL and scope derivation,
     - reflection job persistence and execution,
     - backend-owned config and policy.

2. **Thin OpenClaw adapter**
   - lives in `index.ts`, `src/backend-client/*`, `src/backend-tools.ts`, and relevant config wiring in `openclaw.plugin.json`;
   - owns:
     - hook registration,
     - tool registration,
     - transport/auth header handoff,
     - retry/backoff and error translation,
     - fail-open vs fail-closed policy at OpenClaw integration boundary;
   - does **not** own storage, retrieval, ranking, ACL, or scope authority.

3. **Local context-engine**
   - lives in `src/context/*`;
   - owns:
     - prompt-time gating for recall/injection,
     - `<relevant-memories>`, `<inherited-rules>`, and `<error-detected>` block rendering,
     - session-local suppression and dedupe state,
     - error-signal exposure rules;
   - consumes backend-authoritative rows only;
   - does **not** derive scopes, widen access, or act as fallback backend.

Canonical architecture sentence:

> Rust backend owns memory/RAG authority; the OpenClaw adapter only integrates runtime hooks/tools and transport; the local context-engine only orchestrates prompt-time context.

## Runtime Contract (Remote-Only)

- Supported runtime requires remote backend authority.
- No supported runtime path may operate with local-authority mode.
- Local-authority runtime code has been removed from active source modules; any residual references must be treated as doc/archive debt only.

## Residual Local-Authority Surfaces (Post Cleanup #3)

Removed in cleanup #2:
- `index.ts` local runtime construction/tool/CLI/auto-capture/local-reflection/startup-backup branches.
- remote-mode parse fallback for `remoteBackend.enabled=false`.
- schema/runtime surfaces in `openclaw.plugin.json` for local authority (`embedding`, `dbPath`, `retrieval`, `scopes`, `mdMirror`, `memoryReflection.storeToLanceDB`).
- first local-authority files/tests: `src/tools.ts`, `src/migrate.ts`, `cli.ts`, `test/cli-smoke.mjs`, `test/migrate-legacy-schema.test.mjs`.

Removed in cleanup #3:
- remaining local-authority modules:
  - `src/store.ts`
  - `src/retriever.ts`
  - `src/embedder.ts`
  - `src/scopes.ts`
  - `src/access-tracker.ts`
- local benchmark harness tied to deleted retriever/store chain:
  - `src/benchmark.ts`
  - `test/benchmark-runner.mjs`
- transitive type coupling from permanent modules:
  - `src/context/auto-recall-orchestrator.ts`
  - `src/context/reflection-prompt-planner.ts`
  - `src/reflection-store.ts`
  - `src/reflection-recall.ts`
  - `src/auto-recall-final-selection.ts`
  - via shared remote-safe types in `src/memory-record-types.ts`.

Residual state:
- no runtime/test import edge remains from active modules to deleted local-authority files.

## Module Boundaries and Data Flow

### Startup (target)
1. `index.ts` parses remote config and builds backend clients.
2. Hook and tool registration remain local to plugin runtime.
3. No local memory store/retriever/embedder initialization occurs.

### Prompt-time recall flow (target)
1. lifecycle hook (`before_agent_start` or `before_prompt_build`) fires in `index.ts`.
2. `src/context/*` planner decides whether to inject prompt context.
3. planner calls adapter-facing recall function.
4. adapter calls Rust backend and receives authoritative rows.
5. `src/context/*` renders prompt blocks locally.

### Mutation flow (target)
1. tools in `src/backend-tools.ts` construct request payloads.
2. adapter resolves runtime principal identity and attaches trusted headers.
3. backend performs ACL/scope/persistence decisions and returns structured results.
4. adapter maps backend errors/results to OpenClaw-facing behavior.

## Interfaces and Contract Ownership

### Backend owns
- recall (generic/reflection) contracts;
- mutation/list/stats contracts;
- reflection job enqueue/status contracts;
- ACL and scope semantics.

### Adapter owns
- runtime identity/context mapping;
- transport retries and idempotency headers;
- error translation and route-level fail-open/fail-closed policy.

### Context-engine owns
- prompt-time planner logic and rendering tags;
- session-local dedupe/suppression/error-signal exposure;
- no authority over scope, ACL, or persistence.

## Security and Reliability Rules

Security:
- backend is sole ACL/scope authority;
- adapter must not accept client scope authority input for memory operations;
- context-engine must not synthesize scope expansion.

Reliability:
- recall and prompt injection paths stay fail-open;
- write/update/delete/enqueue paths surface failures clearly (fail-closed where required);
- reflection enqueue remains non-blocking for `/new` and `/reset`.

## Phase 4 Sign-Off Invariants (2026-03-16)

- remote-only runtime remains hard-enforced in `index.ts` (`parsePluginConfig` requires `remoteBackend.enabled=true` plus `baseURL` and `authToken`);
- runtime principal identity remains mandatory for writes/management/enqueue via `src/backend-tools.ts` + `src/backend-client/runtime-context.ts` (`MissingRuntimePrincipalError` path retained);
- tool contracts remain backend-authoritative for scope (remote tools do not accept client `scope` input; scope is backend-owned);
- reflection `/new` and `/reset` remain remote enqueue flows and do not reintroduce local persistence branches.

## Verification Strategy

Planning/doc hygiene:
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/remote-authority-reset`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/remote-authority-reset README.md`
- `git diff --check`

Implementation/deletion phases:
- phase 2 gates: remote-only parse/schema contract tests.
- phase 3 gates: module deletion + import graph convergence + boundary tests.
- phase 4 gates: full regression + release closeout evidence.

See:
- `remote-only-local-authority-removal-plan.md`
- `task-plans/phase-2-remote-authority-reset.md`
- `task-plans/phase-3-remote-authority-reset.md`
- `task-plans/phase-4-remote-authority-reset.md`
