# memory-lancedb-pro · OpenClaw 插件

面向 [OpenClaw](https://github.com/openclaw/openclaw) 的增强长期记忆插件，支持两种运行时权威模式：

- 本地 LanceDB 权威（默认）
- 远程后端权威（通过 `remoteBackend.*`，高级 / MVP 路径）

[English](README.md) | **简体中文**

## 概览

`memory-lancedb-pro` 现在是明确的双模式架构：

- **本地模式**：插件进程内负责记忆权威（LanceDB + Embedding + 检索）。
- **远程模式**：插件仍在本地完成 prompt 编排和 hooks，但把记忆数据面的权威委托给远程 HTTP 后端。

远程模式适合高级部署与契约化集成。该路径已支持，但能力上限取决于你的后端实现。

## 架构

```text
OpenClaw runtime
  |
  +-- index.ts
      （入口、配置解析/校验、hook 注册、模式切换）
      |
      +-- src/context/*
      |   （本地 prompt 编排边界）
      |   - auto-recall orchestrator
      |   - reflection prompt planner
      |   - prompt block renderer / session exposure state
      |
      +-- 本地 LanceDB 权威路径
      |   - src/store.ts
      |   - src/embedder.ts
      |   - src/retriever.ts
      |   - src/tools.ts
      |   - cli.ts（memory-pro）
      |
      +-- 远程权威路径
          - src/backend-client/*
          - src/backend-tools.ts
          - HTTP 数据面端点
            (/v1/recall/*, /v1/memories/*, /v1/reflection/jobs)
          - 可选 Rust 后端 backend/（MVP 参考实现）
```

以上拆分是当前 memory 插件内部架构，不代表已经发布独立的 ContextEngine 插件。

## 安装

1. 推荐克隆到 OpenClaw workspace 插件目录：

```bash
git clone https://github.com/win4r/memory-lancedb-pro.git ~/.openclaw/workspace/plugins/memory-lancedb-pro
cd ~/.openclaw/workspace/plugins/memory-lancedb-pro
npm install
```

2. 在 OpenClaw 配置中加载并绑定 memory slot：

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

3. 重启并验证：

```bash
openclaw gateway restart
openclaw plugins info memory-lancedb-pro
openclaw config get plugins.slots.memory
```

## 选择你的模式

下面任一配置块可作为 `plugins.entries.memory-lancedb-pro.config`。

### 本地 LanceDB 模式（默认）

- `embedding` 必填。
- `dbPath` 生效（LanceDB 存储路径）。

```json
{
  "embedding": {
    "apiKey": "${OPENAI_API_KEY}",
    "model": "text-embedding-3-small"
  },
  "dbPath": "~/.openclaw/memory/lancedb-pro"
}
```

### 远程后端模式（高级 / MVP）

- 切换权威时需要 `remoteBackend.enabled`、`remoteBackend.baseURL`、`remoteBackend.authToken`。
- 远程权威开启后，本地 `embedding` 配置可选且不会被使用。

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

## 远程模式主体身份约束

远程数据面调用要求真实运行时主体身份（`userId` + `agentId`）。

实际行为：

- 召回类读取在身份缺失时会跳过（例如 auto-recall / reflection recall / `memory_recall`）。
- 写入与入队在身份缺失时会失败关闭（例如 `memory_store`、`memory_update`、`memory_forget`、auto-capture、reflection job enqueue）。
- `memory_list` / `memory_stats` 也要求主体身份，缺失时会返回明确错误。
- 远程工具不接受客户端 `scope` 输入；可见性与目标 scope 由后端权威决定。

## Reflection 命令契约（`/new` 与 `/reset`）

- `command:new` 与 `command:reset` 统一使用归一化触发器契约（`new` 或 `reset`）。
- 本地权威路径：同步生成 reflection（失败开放）、可选本地持久化、可选 handoff note 注入。
- 远程权威路径：使用同一 actor 契约异步入队 reflection job（不阻塞命令返回）。
- 两条路径在 hook 结束后都会清理 reflection prompt 的 session 状态。

## 功能矩阵

| 能力 | 本地 LanceDB 权威 | 远程后端权威 |
|---|---|---|
| 混合检索（vector + BM25） | ✅ `src/retriever.ts` 原生管线 | ⚠️ 由后端实现定义 |
| Rerank 提供商（`jina`、`siliconflow`、`voyage`、`pinecone`、`vllm`） | ✅ 通过 `retrieval.*` | ⚠️ 由后端实现定义 |
| 多 Scope 隔离 | ✅ `global`、`agent:*`、`custom:*`、`project:*`、`user:*` | ⚠️ 后端 caller-scope 权威（不接收客户端 `scope`） |
| Auto-capture / Auto-recall hooks | ✅ | ✅（经后端 API 转发） |
| Session 策略（`memoryReflection` / `systemSessionMemory` / `none`） | ✅ | ✅ |
| `memoryReflection` 流程 | ✅ 本地召回 + 本地持久化选项 | ✅ 本地编排 + 后端 reflection 召回/入队权威 |
| `selfImprovement` 工具与提醒 | ✅ | ✅ |
| `mdMirror` | ✅ Markdown 双写 | ❌ 本地权威特性 |
| CLI / 管理工具 | ✅ `memory-pro` CLI + 可选管理工具 | ⚠️ `memory-pro` CLI 禁用；仍可通过远程工具暴露管理能力 |
| 远程后端模式 | ❌ | ✅ 通过 `remoteBackend.*` |

## 工具与 CLI

核心工具：

- `memory_recall`
- `memory_store`
- `memory_forget`
- `memory_update`
- `self_improvement_log`

可选管理工具（`enableManagementTools: true`）：

- `memory_list`
- `memory_stats`
- `self_improvement_review`
- `self_improvement_extract_skill`

`memory-pro` CLI（仅本地模式）包含例如：

- `list`、`search`、`stats`、`delete`、`delete-bulk`
- `export`、`import`、`reembed`、`migrate`
- `reindex-fts`、`benchmark`

## 可选 Rust 后端

`backend/` 提供远程模式可选的 Rust 参考后端（MVP 契约实现）。

示例启动命令：

```bash
cargo run --manifest-path backend/Cargo.toml -- --config /path/to/backend.toml
```

契约文档：

- `docs/remote-memory-backend/remote-memory-backend-contracts.md`

## 常见问题 / 排错

- `embedding config is required when remoteBackend is disabled (local mode)`：
  - 本地模式需补齐 `embedding` 配置。
- `remoteBackend.baseURL/authToken is required when remoteBackend is enabled`：
  - 远程模式需同时提供这两个字段。
- 远程模式出现 `missing runtime principal` 警告/错误：
  - 确保运行时上下文包含 `userId` 与 `agentId`。
- `Vector dimension mismatch`：
  - 保持 `embedding.dimensions` 与已有向量维度一致，或切换新的 `dbPath`。
- `memory-pro` CLI 不可用：
  - 远程权威模式下这是预期行为。

## 参考

- 配置 Schema：`openclaw.plugin.json`
- 本地长文本分块说明：`docs/long-context-chunking.md`
- context-engine-split 文档索引：`docs/context-engine-split/README.md`
- remote-memory-backend 文档索引：`docs/remote-memory-backend/README.md`
- 远程后端契约：`docs/remote-memory-backend/remote-memory-backend-contracts.md`
- 历史执行归档：`docs/archive/`

## License

MIT
