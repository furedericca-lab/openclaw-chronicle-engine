# Chronicle Engine · OpenClaw Plugin

Enhanced long-term memory plugin for [OpenClaw](https://github.com/openclaw/openclaw), with remote backend authority as the only supported runtime authority:

- Remote backend authority via `remoteBackend.*` (only supported runtime authority)
- Local adapter + context-engine for integration and prompt-time orchestration only

**English** | [简体中文](README_CN.md)

## Overview

`Chronicle Engine` follows one canonical architecture:

- **Remote authority (canonical)**: backend HTTP endpoints own storage/retrieval/ranking/scope/reflection authority.
- **Local adapter/context-engine**: this plugin keeps hook/tool registration and prompt-time orchestration only.

Keep `remoteBackend.enabled=true` for supported deployments.

## Architecture

```text
OpenClaw runtime
  |
  +-- index.ts
      (entrypoint, config parse/validation, hook registration, remote-only runtime wiring)
      |
      +-- src/context/*
      |   (local prompt orchestration seams)
      |   - auto-recall orchestrator
      |   - reflection prompt planner
      |   - prompt block renderer / session exposure state
      |
      +-- Remote authority path (canonical)
          - src/backend-client/*
          - src/backend-tools.ts
          - HTTP data-plane endpoints
            (/v1/recall/*, /v1/memories/*, /v1/reflection/jobs)
          - Rust backend under backend/ (current authority implementation)
```

This split is internal to the current memory plugin. It does **not** mean a standalone ContextEngine plugin is shipped.

## Install

1. Clone into your OpenClaw workspace plugin path (recommended):

```bash
git clone https://github.com/furedericca-lab/openclaw-chronicle-engine.git ~/.openclaw/workspace/plugins/openclaw-chronicle-engine
cd ~/.openclaw/workspace/plugins/openclaw-chronicle-engine
npm install
```

2. Load and slot the plugin in OpenClaw config:

```json
{
  "plugins": {
    "load": {
      "paths": ["plugins/openclaw-chronicle-engine"]
    },
    "entries": {
      "openclaw-chronicle-engine": {
        "enabled": true,
        "config": {}
      }
    },
    "slots": {
      "memory": "openclaw-chronicle-engine"
    }
  }
}
```

3. Restart and verify:

```bash
openclaw gateway restart
openclaw plugins info openclaw-chronicle-engine
openclaw config get plugins.slots.memory
```

## Canonical Runtime Configuration (Remote Authority Only)

Use this block as `plugins.entries.openclaw-chronicle-engine.config` for supported runtime behavior.

### Remote backend authority (canonical)

- `remoteBackend.enabled`, `remoteBackend.baseURL`, and `remoteBackend.authToken` are required.

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

## Remote-Authority Principal Contract

Remote data-plane calls require real runtime principal identity (`userId` + `agentId`).

Practical behavior:

- Recall-style reads skip when identity is unavailable (for example auto-recall / reflection recall / `memory_recall`).
- Writes and enqueue flows fail closed when identity is unavailable (for example `memory_store`, `memory_update`, `memory_forget`, auto-capture, reflection job enqueue).
- `memory_list` / `memory_stats` also require principal identity and return explicit errors if missing.
- Remote tools do not accept client `scope` input; caller visibility and target scope stay backend-owned.

## Reflection Command Contract (`/new` and `/reset`)

- `command:new` and `command:reset` use one normalized trigger contract (`new` or `reset`) before running reflection flow.
- Reflection is queued through remote backend jobs (non-blocking) using the same actor contract.
- Prompt-session reflection state is cleared after hook execution.

## Feature Matrix

| Capability | Supported runtime behavior |
|---|---|
| Hybrid retrieval (vector + BM25) | ⚠️ Backend-defined authority |
| Rerank providers (`jina`, `siliconflow`, `voyage`, `pinecone`, `vllm`) | ⚠️ Backend-defined authority |
| Multi-scope isolation | ⚠️ Backend-owned caller scope (no client `scope` input) |
| Auto-capture / auto-recall hooks | ✅ Local orchestration + backend APIs |
| Session strategy (`memoryReflection` / `systemSessionMemory` / `none`) | ✅ |
| `memoryReflection` prompt flows | ✅ Local orchestration + backend reflection recall/enqueue authority |
| `selfImprovement` tools and reminders | ✅ |
| CLI / management tooling | ✅ Remote-backed tools only (`memory_*`, optional management set) |
| Remote backend authority | ✅ via `remoteBackend.*` |

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

Local `memory-pro` CLI and migration commands have been removed.
Use remote-backed tools (`memory_*`) for runtime operations.

## Rust Backend

`backend/` contains the current Rust remote backend implementation used by the canonical remote-authority contract.

Example start command:

```bash
cargo run --manifest-path backend/Cargo.toml -- --config /path/to/backend.toml
```

Contract reference:

- `docs/runtime-architecture.md`

## Troubleshooting

- `remoteBackend.baseURL/authToken is required when remoteBackend is enabled`:
  - set both required fields in plugin config.
- `missing runtime principal` warnings/errors in remote-authority mode:
  - ensure runtime context provides `userId` and `agentId`.

## References

- Config schema: `openclaw.plugin.json`
- Current docs index: `docs/README.md`
- Runtime architecture: `docs/runtime-architecture.md`
- Historical execution artifacts and superseded plans: `docs/archive/`

## License

MIT
