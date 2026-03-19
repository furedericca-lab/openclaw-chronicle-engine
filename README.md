# Chronicle Engine · OpenClaw Memory Plugin

Remote-authority memory for [OpenClaw](https://github.com/openclaw/openclaw), with a Rust backend as the only supported runtime authority.

**English** | [简体中文](README_CN.md)

## 1. What This Plugin Is

Chronicle Engine is not a local memory database embedded inside the plugin anymore.

The supported model is:

- the **Rust backend** owns memory authority
- the **plugin** owns OpenClaw integration and prompt-time orchestration
- the **client/runtime** provides authenticated principal identity

In practical terms:

- storage happens in the backend
- retrieval and ranking happen in the backend
- scope and ACL happen in the backend
- prompt injection and session-local dedupe stay in the plugin
- backend-facing recall filter semantics stay in the backend; the plugin only keeps prompt-time orchestration and rendering

## 2. Architecture At A Glance

```text
                      +--------------------------------------+
                      |            OpenClaw Runtime          |
                      |   hooks, tools, lifecycle events     |
                      +-------------------+------------------+
                                          |
                                          v
                     +----------------------------------------+
                     |           Chronicle Engine Plugin      |
                     | index.ts                               |
                     | src/backend-client/*                   |
                     | src/backend-tools.ts                   |
                     | src/context/*                          |
                     +-------------------+--------------------+
                                         |
                     data-plane HTTP     |  prompt-time orchestration
                     + auth headers      |  session-local state only
                                         |
                                         v
                  +---------------------------------------------+
                  |           Rust Remote Memory Backend        |
                  | backend/src/*                               |
                  | LanceDB + SQLite                            |
                  | retrieval / ranking / scope / ACL / jobs    |
                  +---------------------------------------------+
```

## 3. Ownership Split

### Backend vs plugin

| Concern | Backend (`backend/src/*`) | Plugin (`index.ts`, `src/backend-client/*`, `src/context/*`) |
|---|---|---|
| Memory persistence | Owns | Does not own |
| Recall candidate generation | Owns | Does not own |
| Ranking / rerank / MMR / decay | Owns | Does not own |
| Scope derivation / ACL | Owns | Must not reconstruct |
| Auto-capture write acceptance + persistence | Owns | Only forwards runtime payloads |
| Behavioral-guidance recall retrieval | Owns | Only plans prompt-time autoRecall guidance injection |
| Distill job execution | Owns | Only enqueues / polls |
| Distill source cleaning / artifact persistence | Owns | Does not own |
| Distill lesson/governance derivation | Owns | Does not own |
| Debug recall / distill status surfaces | Owns | Only calls typed client adapters |
| Hook registration | Does not own | Owns |
| Backend DTO transport adapters | Does not own | Owns |
| Prompt block rendering | Does not own | Owns |
| Session-local exposure suppression | Does not own | Owns |
| Fail-open vs fail-closed route behavior | Shared contract, backend-enforced + plugin-handled | Owns runtime behavior at hook/tool boundary |

### Old mental model vs current model

| Question | Old local-authority model | Current supported model |
|---|---|---|
| Where is the source of truth? | Local TS modules | Remote Rust backend |
| Can the plugin decide scopes? | Historically yes | No |
| Can the plugin rank final backend-visible rows authoritatively? | Historically yes | No |
| Can the plugin shape prompt injection locally? | Yes | Yes |
| Is there a supported local fallback memory engine? | Historically yes | No |

## 4. Request Flow

### Generic recall

```text
User prompt
  -> OpenClaw hook
    -> Chronicle Engine planner
      -> backend client
        -> POST /v1/recall/generic
          -> backend retrieves + ranks + filters
            -> plugin receives authoritative rows
              -> local prompt block rendering
                -> <relevant-memories> injected into prompt
```

### Cadence-driven distill flow

```text
agent_end
  -> plugin appends ordered transcript rows
    -> backend persists session transcript
      -> every distill.everyTurns user turns
        -> plugin enqueues POST /v1/distill/jobs
          -> backend derives distill artifacts from session trajectory
            -> later recall/injection can read persisted rows and artifacts
```

### Distill job flow

```text
distill request
  -> plugin/backend client
    -> POST /v1/distill/jobs
        -> backend validates actor + source + mode
        -> backend enqueues async distill job
          -> backend cleans transcript/messages and builds deterministic span/window candidates
            -> backend persists English distill artifacts
              -> optional memory-row persistence
                -> GET /v1/distill/jobs/{jobId} to inspect status/result
```

## 5. Old TS RAG vs Current Rust Remote RAG

### Capability comparison

| Capability | Old TS-heavy chain | Current Rust remote chain | Current status |
|---|---|---|---|
| Persistence authority | Local TS modules owned writes and storage | Rust backend owns writes and storage | Replaced |
| Vector retrieval | Local TS implementation | Rust backend | Replaced |
| Lexical / BM25-style retrieval | Local TS implementation | Rust backend | Replaced |
| Hybrid merge | Local TS implementation | Rust backend | Replaced |
| Rerank | Local TS implementation | Rust backend | Replaced |
| Rerank fallback / key rotation | Local TS implementation | Rust backend | Replaced |
| Recency / decay / length weighting | Local TS implementation | Rust backend | Replaced |
| Access reinforcement time-decay | Historical TS-side capability | Rust backend | Present |
| Diversity / MMR | Historical TS-side capability | Rust backend | Present |
| Behavioral-guidance recall authority | Local TS + local persistence path | Rust backend recall path with plugin-side autoRecall behavioral rendering | Replaced |
| Command-triggered trajectory-derived generation | Local/plugin-coupled execution | Removed; cadence-driven distill is the only supported generation path | Removed |
| Distill async jobs | Historical sidecar/example pipeline | Rust backend distill jobs | Present, backend-native deterministic runtime |
| Scope derivation / ACL | Local TS participation existed historically | Rust backend only | Replaced |
| Inspectable retrieval trace | Historical TS had thicker telemetry objects | Rust backend debug trace routes | Acceptable parity, not 1:1 shape recreation |
| Prompt injection rendering | Local TS | Local TS | Intentionally retained |
| Session-local exposure suppression | Local TS | Local TS | Intentionally retained |
| Final generic auto-recall trimming | Local TS | Local TS over backend-returned rows | Limited to direct prompt injection truncation |

### What was not recreated 1:1

| Historical TS shape | Current replacement |
|---|---|
| Thick local telemetry object model | backend debug trace routes with structured stages/fallback/counts/final row ids |
| Local authority ranking chain | backend-owned ranking chain |
| Local scope authority helpers | backend principal + scope authority |

## 6. Has Old TS Been Fully Removed?

No, but the answer needs precision:

- **old TS local-authority runtime**: removed
- **TS prompt-local orchestration**: intentionally retained

### Removed old local-authority modules

| Removed path | Why removed |
|---|---|
| `src/store.ts` | local persistence authority removed |
| `src/retriever.ts` | local retrieval authority removed |
| `src/embedder.ts` | local embedding authority removed |
| `src/chunker.ts` | unused local chunking helper removed after import-proof showed no active runtime or test dependency |
| `src/tools.ts` | old local-authority tool path removed |
| `src/migrate.ts` | old local migration path removed |
| `src/scopes.ts` | local scope authority removed |
| `src/access-tracker.ts` | old local access-metadata authority removed |
| `cli.ts` | old local CLI path removed |

### Retained TS modules and why they still exist

| Retained path | Why it remains |
|---|---|
| `src/context/*` | prompt-time orchestration only |
| `src/context/recall-engine.ts` | local gating / dedupe / exposure-state helpers |
| `src/context/adaptive-retrieval.ts` | prompt-side retrieval trigger heuristic |

### Practical interpretation

If the question is:

- “Is the old TS authority chain still alive?” -> **No**
- “Does the repo still contain TS files related to recall/behavioral guidance?” -> **Yes, intentionally, for prompt-local orchestration and tests**

## 7. Runtime Rules That Matter

### Principal identity contract

Remote data-plane calls require real runtime principal identity:

- `userId`
- `agentId`

Behavior by path:

| Path type | If principal identity is missing |
|---|---|
| Recall / prompt injection | Skip fail-open |
| Write / update / delete | Fail closed |
| Auto-capture | Fail closed |
| List / stats | Fail closed |
| Distill enqueue | Fail closed |

### Scope contract

The plugin does **not** submit a target `scope`.

That means:

- callers do not choose target scope in tool payloads
- backend derives and enforces visibility
- client-side scope reconstruction is not part of the supported architecture

## 8. Supported Features

| Capability | Status | Notes |
|---|---|---|
| Remote backend authority | Yes | Required for supported runtime behavior |
| Hybrid retrieval | Yes | Backend-owned |
| Provider-backed embeddings | Yes | Backend-owned |
| Rerank + fallback | Yes | Backend-owned |
| Time decay + access reinforcement | Yes | Backend-owned |
| Diversity / MMR | Yes | Backend-owned |
| Auto-recall prompt injection | Yes | Local orchestration over backend recall |
| AutoRecall behavioral-guidance planning | Yes | Read-only behavioral recall in backend, prompt-local guidance injection in plugin |
| Distill job enqueue + polling | Yes | Backend-owned async job surface |
| Distill inline-message cleaning + artifact persistence | Yes | Backend-owned execution path |
| Distill `session-transcript` source | Yes | Backend-owned transcript persistence + async distill execution |
| Automatic distill every N user turns | Yes | Runtime cadence over backend-native `session-transcript` jobs |
| `session-lessons` mode | Yes | Owns lesson, cause, fix, prevention, stable decision, and durable practice extraction |
| `governance-candidates` mode | Yes | Owns worth-promoting learnings, skill extraction candidates, and AGENTS/SOUL/TOOLS promotion candidates |
| Distill artifact subtypes | Yes | `follow-up-focus` and `next-turn-guidance` replace separate derived/open-loop reflection persistence |
| `memory_store` / `memory_update` / `memory_forget` | Yes | Remote-backed |
| `memory_list` / `memory_stats` | Yes | Optional management tools |
| `memory_distill_enqueue` / `memory_distill_status` | Yes | Optional management tools for caller-scoped backend distill jobs |
| `memory_recall_debug` | Yes | Optional management/debug tool for explicit recall trace inspection |
| Local `memory-pro` CLI | No | Removed |
| Supported local-authority runtime | No | Removed |

## 9. Distill: Old Sidecar vs Current Backend-Native Direction

| Concern | Historical `jsonl_distill.py` / sidecar pipeline | Current backend-native direction |
|---|---|---|
| Job ownership | External script + worker | Rust backend job surface |
| Source preprocessing | Script-local filtering/cleanup | Backend cleanup/filtering pipeline |
| Reduction quality | Sidecar reduction pipeline | Deterministic Rust turns-stage lesson reducer |
| Persistence | External import back into storage | Backend-owned artifacts and optional memory persistence |
| Status inspection | Queue files / external worker logs | `GET /v1/distill/jobs/{jobId}` |
| Runtime authority | Not canonical anymore | Canonical direction |

Current runtime shape:

- runtime appends ordered transcript rows to backend on `agent_end`
- runtime may optionally enqueue one backend-native `session-transcript` distill job every configured `distill.everyTurns` user turns
- backend resolves the source rows, cleans them, builds deterministic span/window candidates, merges overlapping evidence, and persists artifacts
- when `persistMode=persist-memory-rows`, backend also persists distilled memory rows from the final artifacts

Current behavior boundary:

- the old `jsonl_distill.py` / example-worker sidecar path has been removed from the active repo runtime
- it is not the supported runtime path
- the supported direction is backend-native distill jobs backed by persisted session transcript rows
- distill summaries in the current runtime are intentionally English-only and deterministic
- optional runtime cadence can enqueue one `session-transcript` distill job every configured `distill.everyTurns` user turns

What current distill is good at:

- deterministic turns-stage lesson extraction without sidecar infrastructure
- multi-message evidence aggregation inside backend reduction windows
- evidence-gated promotion of `stable decision` / `durable practice` rather than single-keyword escalation
- stable artifacts and optional memory persistence under the same caller-scoped backend authority model
- keeping all new-learning writes under `session-lessons` and `governance-candidates`

What current distill is intentionally not:

- language-adaptive extraction
- a separate non-distill generation pipeline
- a restored queue-file / worker / `memory-pro import` architecture

## 10. Debuggability

Chronicle Engine now has two layers of observability:

| Surface | Purpose | Contract stability |
|---|---|---|
| `/v1/recall/*` | ordinary runtime recall | stable data-plane DTOs |
| `/v1/debug/recall/*` | inspect retrieval traces | explicit debug surface, separate from ordinary DTO rows |

Important boundary:

- ordinary recall DTOs do **not** expose raw score-breakdown internals
- debug trace routes exist so debugging gets richer visibility without bloating runtime contracts
- `memory_recall_debug` is the management-gated tool surface for those debug routes

## 11. Install

### Clone into the OpenClaw plugin workspace

```bash
git clone https://github.com/furedericca-lab/openclaw-chronicle-engine.git ~/.openclaw/workspace/plugins/openclaw-chronicle-engine
cd ~/.openclaw/workspace/plugins/openclaw-chronicle-engine
npm install
```

### Slot it as the `memory` plugin

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

### Verify the slot

```bash
openclaw gateway restart
openclaw plugins info openclaw-chronicle-engine
openclaw config get plugins.slots.memory
```

## 12. Minimal Supported Configuration

Use this as `plugins.entries.openclaw-chronicle-engine.config`.

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

Required fields:

| Field | Required | Meaning |
|---|---|---|
| `remoteBackend.enabled` | Yes | Must be `true` |
| `remoteBackend.baseURL` | Yes | Backend base URL |
| `remoteBackend.authToken` | Yes | Runtime bearer token |
| `timeoutMs` | No | Request timeout |
| `maxRetries` | No | Transport retry count |
| `retryBackoffMs` | No | Retry backoff |

Cutover note:

- `1.0.0-beta.0` removes migration-only config aliases.
- Only `sessionStrategy: "autoRecall" | "systemSessionMemory" | "none"` is supported.
- Use `autoRecallBehavioral.*` as the canonical behavioral-guidance config surface.
- Use `governance.*` for backlog/review workflow configuration.
- Legacy pre-closeout config aliases are rejected.

## 13. Tools

### Core tools

- `memory_recall`
- `memory_store`
- `memory_forget`
- `memory_update`
- `governance_log`

### Optional management tools

Enable `enableManagementTools: true` to expose:

- `memory_list`
- `memory_stats`
- `memory_distill_enqueue`
- `memory_distill_status`
- `memory_recall_debug`
- `governance_review`
- `governance_extract_skill`

Management/debug tools stay caller-scoped and require runtime principal identity. They are not available as anonymous local fallbacks.

### Backend client management/debug surfaces

The plugin client also has backend job adapters for:

- distill jobs
- recall debug traces (`generic` and `behavioral`)

## 14. Repository Layout

```text
backend/                  Rust backend implementation
docs/runtime-architecture.md
docs/archive/             historical plans and closed scopes
src/backend-client/*      transport + DTO adapter
src/backend-tools.ts      tool bridge
src/context/*             prompt-time orchestration
test/*                    plugin-side tests
```

## 15. Testing

### Plugin tests

```bash
npm test
```

### Backend tests

```bash
cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture
```

## 16. Common Misunderstandings

### “Is this still a local LanceDB plugin?”

No. The supported runtime model is remote authority only.

### “Does `src/context/*` mean local authority still exists?”

No. `src/context/*` is prompt-time orchestration:

- when to recall
- how to render injected blocks
- how to suppress repeated exposure in the same session

It is not backend ownership.

### “Do old config aliases still work?”

No. Use the current schema names only:

- `sessionStrategy`
- `autoRecallBehavioral.*`
- `governance.*`

### “Does distill still mean running the old `jsonl_distill.py` sidecar?”

No. That sidecar path has been removed from the active runtime and the repo.

The supported direction is:

- backend-native distill jobs
- backend-owned status
- backend-owned artifacts
- backend-owned session transcript persistence and replay-safe source resolution

The old sidecar/example pipeline is not the canonical runtime path.

## 17. References

- Runtime architecture: `docs/runtime-architecture.md`
- Docs index: `docs/README.md`
- Historical execution and closed scopes: `docs/archive/`
- Plugin schema: `openclaw.plugin.json`

## License

MIT
