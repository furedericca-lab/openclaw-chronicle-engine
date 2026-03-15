# memory-lancedb-pro · OpenClaw Plugin

Enhanced long-term memory plugin for [OpenClaw](https://github.com/openclaw/openclaw), with two runtime authority modes:

- Local LanceDB authority (default)
- Remote backend authority via `remoteBackend.*` (advanced / MVP path)

**English** | [简体中文](README_CN.md)

## Overview

`memory-lancedb-pro` now runs in a clear split model:

- **Local mode**: this plugin owns memory authority in-process (LanceDB + embedding + retrieval).
- **Remote mode**: this plugin keeps prompt orchestration and hooks locally, but delegates memory authority to remote HTTP endpoints.

Remote mode is intended for advanced deployments and contract-based integration. It is supported, but backend capability parity depends on your backend implementation.

## Architecture

```text
OpenClaw runtime
  |
  +-- index.ts
      (entrypoint, config parse/validation, hook registration, mode switch)
      |
      +-- src/context/*
      |   (local prompt orchestration seams)
      |   - auto-recall orchestrator
      |   - reflection prompt planner
      |   - prompt block renderer / session exposure state
      |
      +-- Local LanceDB authority path
      |   - src/store.ts
      |   - src/embedder.ts
      |   - src/retriever.ts
      |   - src/tools.ts
      |   - cli.ts (memory-pro)
      |
      +-- Remote authority path
          - src/backend-client/*
          - src/backend-tools.ts
          - HTTP data-plane endpoints
            (/v1/recall/*, /v1/memories/*, /v1/reflection/jobs)
          - optional Rust backend under backend/ (MVP reference)
```

This split is internal to the current memory plugin. It does **not** mean a standalone ContextEngine plugin is shipped.

## Install

1. Clone into your OpenClaw workspace plugin path (recommended):

```bash
git clone https://github.com/win4r/memory-lancedb-pro.git ~/.openclaw/workspace/plugins/memory-lancedb-pro
cd ~/.openclaw/workspace/plugins/memory-lancedb-pro
npm install
```

2. Load and slot the plugin in OpenClaw config:

```json
{
  "plugins": {
    "load": {
      "paths": ["plugins/memory-lancedb-pro"]
    },
    "entries": {
      "memory-lancedb-pro": {
        "enabled": true,
        "config": {}
      }
    },
    "slots": {
      "memory": "memory-lancedb-pro"
    }
  }
}
```

3. Restart and verify:

```bash
openclaw gateway restart
openclaw plugins info memory-lancedb-pro
openclaw config get plugins.slots.memory
```

## Choose Your Mode

Use one of the following blocks as `plugins.entries.memory-lancedb-pro.config`.

### Local LanceDB mode (default)

- `embedding` is required.
- `dbPath` is relevant (LanceDB location).

```json
{
  "embedding": {
    "apiKey": "${OPENAI_API_KEY}",
    "model": "text-embedding-3-small"
  },
  "dbPath": "~/.openclaw/memory/lancedb-pro"
}
```

### Remote backend mode (advanced / MVP)

- `remoteBackend.enabled`, `remoteBackend.baseURL`, and `remoteBackend.authToken` are required for authority switch.
- Local `embedding` config is optional and not used when remote authority is active.

```json
{
  "remoteBackend": {
    "enabled": true,
    "baseURL": "http://127.0.0.1:8080",
    "authToken": "${MEMORY_BACKEND_TOKEN}",
    "timeoutMs": 10000,
    "maxRetries": 1,
    "retryBackoffMs": 250
  }
}
```

## Remote Mode Principal Contract

Remote data-plane calls require real runtime principal identity (`userId` + `agentId`).

Practical behavior:

- Recall-style reads skip when identity is unavailable (for example auto-recall / reflection recall / `memory_recall`).
- Writes and enqueue flows fail closed when identity is unavailable (for example `memory_store`, `memory_update`, `memory_forget`, auto-capture, reflection job enqueue).
- `memory_list` / `memory_stats` also require principal identity and return explicit errors if missing.
- Remote tools do not accept client `scope` input; caller visibility and target scope stay backend-owned.

## Reflection Command Contract (`/new` and `/reset`)

- `command:new` and `command:reset` use one normalized trigger contract (`new` or `reset`) before running reflection flow.
- Local authority path: generates reflection output inline (fail-open), optional local persistence, and optional handoff note injection.
- Remote authority path: enqueues async reflection jobs (non-blocking) using the same actor contract.
- Both paths clear reflection prompt-session state after hook execution.

## Feature Matrix

| Capability | Local LanceDB authority | Remote backend authority |
|---|---|---|
| Hybrid retrieval (vector + BM25) | ✅ Native pipeline in `src/retriever.ts` | ⚠️ Backend-defined |
| Rerank providers (`jina`, `siliconflow`, `voyage`, `pinecone`, `vllm`) | ✅ Via `retrieval.*` | ⚠️ Backend-defined |
| Multi-scope isolation | ✅ `global`, `agent:*`, `custom:*`, `project:*`, `user:*` | ⚠️ Backend-owned caller scope (no client `scope` input) |
| Auto-capture / auto-recall hooks | ✅ | ✅ (transported via backend APIs) |
| Session strategy (`memoryReflection` / `systemSessionMemory` / `none`) | ✅ | ✅ |
| `memoryReflection` prompt flows | ✅ local recall + local persistence options | ✅ local orchestration + backend reflection recall/enqueue authority |
| `selfImprovement` tools and reminders | ✅ | ✅ |
| `mdMirror` | ✅ dual-write to Markdown | ❌ local-authority feature |
| CLI / management tooling | ✅ `memory-pro` CLI + optional management tools | ⚠️ `memory-pro` CLI disabled; management tools still available via remote tools |
| Remote backend mode | ❌ | ✅ via `remoteBackend.*` |

## Tools and CLI

Core tools:

- `memory_recall`
- `memory_store`
- `memory_forget`
- `memory_update`
- `self_improvement_log`

Optional management tools (`enableManagementTools: true`):

- `memory_list`
- `memory_stats`
- `self_improvement_review`
- `self_improvement_extract_skill`

`memory-pro` CLI (local mode only) includes commands such as:

- `list`, `search`, `stats`, `delete`, `delete-bulk`
- `export`, `import`, `reembed`, `migrate`
- `reindex-fts`, `benchmark`

## Optional Rust Backend

`backend/` contains an optional Rust reference backend for remote mode (MVP contract implementation).

Example start command:

```bash
cargo run --manifest-path backend/Cargo.toml -- --config /path/to/backend.toml
```

Contract reference:

- `docs/remote-memory-backend/remote-memory-backend-contracts.md`

## Troubleshooting

- `embedding config is required when remoteBackend is disabled (local mode)`:
  - add `embedding` block in local mode.
- `remoteBackend.baseURL/authToken is required when remoteBackend is enabled`:
  - fill both fields in remote mode.
- `missing runtime principal` warnings/errors in remote mode:
  - ensure runtime context provides `userId` and `agentId`.
- `Vector dimension mismatch`:
  - keep `embedding.dimensions` aligned with existing DB vectors, or use a new `dbPath`.
- `memory-pro` CLI unavailable:
  - expected in remote-authority mode.

## References

- Config schema: `openclaw.plugin.json`
- Local chunking notes: `docs/long-context-chunking.md`
- Context-engine split docs index: `docs/context-engine-split/README.md`
- Remote backend docs index: `docs/remote-memory-backend/README.md`
- Remote backend contracts: `docs/remote-memory-backend/remote-memory-backend-contracts.md`
- Historical execution artifacts archive: `docs/archive/`

## License

MIT
