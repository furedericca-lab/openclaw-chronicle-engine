# Chronicle Engine · OpenClaw 插件

面向 [OpenClaw](https://github.com/openclaw/openclaw) 的增强长期记忆插件，以远程后端权威作为唯一受支持的运行时权威：

- 远程后端权威（通过 `remoteBackend.*`，唯一受支持运行形态）
- 本地适配层 + 上下文引擎仅负责集成与 prompt-time 编排

[English](README.md) | **简体中文**

## 概览

`Chronicle Engine` 采用单一规范架构：

- **远程权威（规范）**：后端 HTTP 端点拥有存储 / 检索 / 排序 / scope / reflection 权威。
- **本地适配层与上下文引擎**：插件仅保留 hook/tool 集成与 prompt-time 编排。

受支持部署应保持 `remoteBackend.enabled=true`。

## 架构

```text
OpenClaw runtime
  |
  +-- index.ts
      （入口、配置解析/校验、hook 注册、仅远程权威运行时接线）
      |
      +-- src/context/*
      |   （本地 prompt 编排边界）
      |   - auto-recall orchestrator
      |   - reflection prompt planner
      |   - prompt block renderer / session exposure state
      |
      +-- 远程权威路径（规范）
          - src/backend-client/*
          - src/backend-tools.ts
          - HTTP 数据面端点
            (/v1/recall/*, /v1/memories/*, /v1/reflection/jobs)
          - Rust 后端 backend/（当前权威实现）
```

以上拆分是当前 memory 插件内部架构，不代表已经发布独立的 ContextEngine 插件。

## 安装

1. 推荐克隆到 OpenClaw workspace 插件目录：

```bash
git clone https://github.com/furedericca-lab/openclaw-chronicle-engine.git ~/.openclaw/workspace/plugins/openclaw-chronicle-engine
cd ~/.openclaw/workspace/plugins/openclaw-chronicle-engine
npm install
```

2. 在 OpenClaw 配置中加载并绑定 memory slot：

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

3. 重启并验证：

```bash
openclaw gateway restart
openclaw plugins info openclaw-chronicle-engine
openclaw config get plugins.slots.memory
```

## 规范运行配置（仅远程权威）

受支持运行请使用下面配置块作为 `plugins.entries.openclaw-chronicle-engine.config`。

### 远程后端权威（规范）

- 需要 `remoteBackend.enabled`、`remoteBackend.baseURL`、`remoteBackend.authToken`。

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

## 远程权威主体身份约束

远程数据面调用要求真实运行时主体身份（`userId` + `agentId`）。

实际行为：

- 召回类读取在身份缺失时会跳过（例如 auto-recall / reflection recall / `memory_recall`）。
- 写入与入队在身份缺失时会失败关闭（例如 `memory_store`、`memory_update`、`memory_forget`、auto-capture、reflection job enqueue）。
- `memory_list` / `memory_stats` 也要求主体身份，缺失时会返回明确错误。
- 远程工具不接受客户端 `scope` 输入；可见性与目标 scope 由后端权威决定。

## Reflection 命令契约（`/new` 与 `/reset`）

- `command:new` 与 `command:reset` 统一使用归一化触发器契约（`new` 或 `reset`）。
- reflection 通过远程后端 job 进行异步入队（不阻塞命令返回）。
- hook 执行后会清理 reflection prompt 的 session 状态。

## 功能矩阵

| 能力 | 受支持运行时行为 |
|---|---|
| 混合检索（vector + BM25） | ⚠️ 由后端权威定义 |
| Rerank 提供商（`jina`、`siliconflow`、`voyage`、`pinecone`、`vllm`） | ⚠️ 由后端权威定义 |
| 多 Scope 隔离 | ⚠️ 后端 caller-scope 权威（不接收客户端 `scope`） |
| Auto-capture / Auto-recall hooks | ✅ 本地编排 + 后端 API |
| Session 策略（`memoryReflection` / `systemSessionMemory` / `none`） | ✅ |
| `memoryReflection` 流程 | ✅ 本地编排 + 后端 reflection 召回/入队权威 |
| `selfImprovement` 工具与提醒 | ✅ |
| CLI / 管理工具 | ✅ 仅远程后端工具（`memory_*` + 可选管理工具） |
| 远程后端权威 | ✅ 通过 `remoteBackend.*` |

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

本地 `memory-pro` CLI 与迁移命令已删除。
运行期请使用远程后端工具（`memory_*`）。

## Rust 后端

`backend/` 提供当前规范远程权威契约对应的 Rust 后端实现。

示例启动命令：

```bash
cargo run --manifest-path backend/Cargo.toml -- --config /path/to/backend.toml
```

契约文档：

- `docs/runtime-architecture.md`

## 常见问题 / 排错

- `remoteBackend.baseURL/authToken is required when remoteBackend is enabled`：
  - 在插件配置中同时设置这两个必填字段。
- 远程权威模式出现 `missing runtime principal` 警告/错误：
  - 确保运行时上下文包含 `userId` 与 `agentId`。

## 参考

- 配置 Schema：`openclaw.plugin.json`
- 当前文档索引：`docs/README.md`
- 运行时架构：`docs/runtime-architecture.md`
- 历史执行与过时文档：`docs/archive/`

## License

MIT
