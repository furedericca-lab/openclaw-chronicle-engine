# Chronicle Engine · OpenClaw 记忆插件

面向 [OpenClaw](https://github.com/openclaw/openclaw) 的远程权威记忆插件，当前唯一受支持的运行形态是：**Rust 后端负责权威，本地插件只做集成与 prompt 编排**。

[English](README.md) | **简体中文**

## 1. 这是什么

Chronicle Engine 已经不是“插件内嵌一个本地记忆数据库”的形态了。

当前规范模型是：

- **Rust 后端** 负责记忆权威
- **插件** 负责 OpenClaw 集成与 prompt-time 编排
- **运行时 / 网关** 负责注入认证后的主体身份

换句话说：

- 存储在后端
- 检索和排序在后端
- scope 与 ACL 在后端
- prompt 注入与 session 级去重保留在插件本地

## 2. 一眼看懂整体架构

```text
                      +--------------------------------------+
                      |            OpenClaw Runtime          |
                      |         hooks / tools / lifecycle    |
                      +-------------------+------------------+
                                          |
                                          v
                     +----------------------------------------+
                     |         Chronicle Engine Plugin        |
                     | index.ts                               |
                     | src/backend-client/*                   |
                     | src/backend-tools.ts                   |
                     | src/context/*                          |
                     +-------------------+--------------------+
                                         |
                     数据面 HTTP         |  prompt-time 编排
                     + 认证头            |  仅保留 session 本地状态
                                         |
                                         v
                  +---------------------------------------------+
                  |           Rust Remote Memory Backend        |
                  | backend/src/*                               |
                  | LanceDB + SQLite                            |
                  | retrieval / ranking / scope / ACL / jobs    |
                  +---------------------------------------------+
```

## 3. 谁负责什么

### Backend 与插件的职责划分

| 事项 | Backend (`backend/src/*`) | 插件 (`index.ts`, `src/backend-client/*`, `src/context/*`) |
|---|---|---|
| 记忆持久化 | 负责 | 不负责 |
| recall candidate 生成 | 负责 | 不负责 |
| ranking / rerank / MMR / decay | 负责 | 不负责 |
| scope derivation / ACL | 负责 | 不允许本地重建 |
| auto-capture 写入接收与持久化 | 负责 | 只负责转发运行时 payload |
| reflection job 执行 | 负责 | 只负责 enqueue |
| distill job 执行 | 负责 | 只负责 enqueue / poll |
| distill source 清洗与 artifact 持久化 | 负责 | 不负责 |
| debug recall / distill status 接口面 | 负责 | 只负责调用 typed client adapter |
| hook 注册 | 不负责 | 负责 |
| backend DTO 传输适配 | 不负责 | 负责 |
| prompt block 渲染 | 不负责 | 负责 |
| session 级曝光抑制 | 不负责 | 负责 |
| fail-open / fail-closed 的插件边界行为 | 与契约共享 | 负责在 hook/tool 边界执行 |

### 旧思路 vs 当前规范思路

| 问题 | 旧本地权威模型 | 当前受支持模型 |
|---|---|---|
| 谁是 source of truth | 本地 TS 模块 | 远程 Rust backend |
| 插件能不能决定 scope | 过去可以 | 不可以 |
| 插件能不能做权威排序 | 过去可以 | 不可以 |
| 插件能不能做 prompt 注入层调整 | 可以 | 可以 |
| 是否存在受支持的本地 fallback backend | 过去存在 | 已移除 |

## 4. 请求是怎么流动的

### 通用 recall

```text
用户 prompt
  -> OpenClaw hook
    -> Chronicle Engine planner
      -> backend client
        -> POST /v1/recall/generic
          -> backend 检索 + 排序 + 过滤
            -> 插件拿到 authoritative rows
              -> 本地渲染 prompt block
                -> 注入 <relevant-memories>
```

### `/new` / `/reset` 的 reflection 流

```text
/new 或 /reset
  -> 插件归一化 trigger
    -> POST /v1/reflection/jobs
      -> backend 异步排队 reflection 任务
        -> 命令立即返回
          -> 后续 recall 再读取已持久化的 reflection rows
```

### distill job 流

```text
distill 请求
  -> 插件 / backend client
    -> POST /v1/distill/jobs
      -> backend 校验 actor + source + mode
        -> backend 异步排队 distill job
          -> worker 路径清洗 transcript / messages
            -> backend 持久化 distill artifacts
              -> 可选写入 memory rows
                -> GET /v1/distill/jobs/{jobId} 查看状态和结果
```

## 5. 旧 TS RAG vs 当前 Rust Remote RAG

### 能力对照表

| 能力 | 旧 TS-heavy 链路 | 当前 Rust remote 链路 | 当前状态 |
|---|---|---|---|
| 持久化权威 | 本地 TS 模块负责 | Rust backend 负责 | 已替换 |
| Vector retrieval | 本地 TS 实现 | Rust backend | 已替换 |
| Lexical / BM25 风格 retrieval | 本地 TS 实现 | Rust backend | 已替换 |
| Hybrid merge | 本地 TS 实现 | Rust backend | 已替换 |
| Rerank | 本地 TS 实现 | Rust backend | 已替换 |
| Rerank fallback / key rotation | 本地 TS 实现 | Rust backend | 已替换 |
| Recency / decay / length weighting | 本地 TS 实现 | Rust backend | 已替换 |
| Access reinforcement time-decay | 历史 TS 能力 | Rust backend | 已具备 |
| Diversity / MMR | 历史 TS 能力 | Rust backend | 已具备 |
| Reflection recall 权威 | 本地 TS + 本地持久化路径 | Rust backend recall 路径 | 已替换 |
| Reflection async jobs | 本地 / 插件耦合执行 | Rust backend enqueue + job tracking | 已替换 |
| Distill async jobs | 历史 sidecar / example 流水线 | Rust backend distill jobs | 已具备初版 backend-native 能力 |
| Scope derivation / ACL | 历史上本地 TS 有参与 | 仅 Rust backend | 已替换 |
| 可检查 retrieval trace | 历史 TS 有更厚 telemetry 对象 | Rust backend debug trace routes | 达到可接受 parity，但不是 1:1 复刻 |
| Prompt 注入渲染 | 本地 TS | 本地 TS | 有意保留 |
| Session-local 暴露抑制 | 本地 TS | 本地 TS | 有意保留 |
| 最终 prompt-only 后选 (`setwise-v2`) | 本地 TS | 本地 TS，作用于 backend 已返回 rows | 有意保留为 prompt-local seam |

### 哪些历史形态没有原样复刻

| 历史 TS 形态 | 当前替代方案 |
|---|---|
| 很厚的本地 telemetry object model | backend debug trace route，返回结构化 stage/fallback/count/final row ids |
| 本地 authority ranking chain | backend 权威 ranking chain |
| 本地 scope authority helpers | backend principal + scope authority |

## 6. 旧 TS 内容都清理了吗？

没有“一刀切全部删除”，但要区分两类：

- **旧 TS 本地权威运行时**：已经清理
- **TS prompt-local 编排代码**：有意保留

### 已删除的旧本地权威模块

| 已删除路径 | 删除原因 |
|---|---|
| `src/store.ts` | 删除本地持久化权威 |
| `src/retriever.ts` | 删除本地 retrieval 权威 |
| `src/embedder.ts` | 删除本地 embedding 权威 |
| `src/chunker.ts` | 经 import-proof 确认后删除；它已不再被活动运行时或测试依赖 |
| `src/tools.ts` | 删除旧本地权威工具路径 |
| `src/migrate.ts` | 删除旧本地迁移路径 |
| `src/scopes.ts` | 删除本地 scope 权威 |
| `src/access-tracker.ts` | 删除旧本地 access-metadata 权威 |
| `cli.ts` | 删除旧本地 CLI 路径 |

### 仍保留的 TS 模块，以及为什么保留

| 保留路径 | 保留原因 |
|---|---|
| `src/context/*` | 仅负责 prompt-time 编排 |
| `src/recall-engine.ts` | 本地 gating / dedupe / exposure-state helper |
| `src/adaptive-retrieval.ts` | prompt 侧 recall 触发启发式 |
| `src/prompt-local-auto-recall-selection.ts` | 对 backend rows 做 prompt-local post-selection |
| `src/prompt-local-topk-setwise-selection.ts` | 服务于保留本地选择 seam 的 prompt-local 工具函数 |
| `src/query-expander.ts` | 保留的 test/reference 词汇扩展 helper；当前受支持运行时不会导入 |
| `src/reflection-store.ts` | 保留的 test/reference reflection 组装 helper；当前受支持运行时不会导入 |
| `test/helpers/reflection-recall-reference.ts` | 保留的 test/reference helper，不是 active backend authority |
| `test/helpers/reflection-recall-selection-reference.ts` | 保留的下游 test/reference 选择 helper |

### 实际应该怎么理解

如果问题是：

- “旧 TS authority 链路还活着吗？” -> **没有**
- “仓库里还有没有与 recall/reflection 相关的 TS 文件？” -> **有，而且是有意保留，用于 prompt-local 编排和测试**

## 7. 运行时规则

### 主体身份约束

远程数据面调用要求真实运行时主体身份：

- `userId`
- `agentId`

不同路径在身份缺失时的行为：

| 路径类型 | 缺失主体身份时的行为 |
|---|---|
| Recall / prompt 注入 | 跳过，fail-open |
| Write / update / delete | 明确失败，fail-closed |
| Auto-capture | 明确失败，fail-closed |
| List / stats | 明确失败，fail-closed |
| Reflection enqueue | 操作失败关闭，但对话流在适用场景下仍保持非阻塞 |

### Scope 约束

插件不会提交目标 `scope`。

这意味着：

- 调用方不能在 tool payload 里指定 target scope
- 可见性由 backend 决定
- 客户端重建 scope 不是受支持架构的一部分

## 8. 当前支持哪些能力

| 能力 | 状态 | 说明 |
|---|---|---|
| 远程后端权威 | 支持 | 受支持运行必需 |
| 混合检索 | 支持 | backend 权威 |
| provider-backed embeddings | 支持 | backend 权威 |
| rerank + fallback | 支持 | backend 权威 |
| time decay + access reinforcement | 支持 | backend 权威 |
| diversity / MMR | 支持 | backend 权威 |
| auto-recall prompt 注入 | 支持 | 本地编排 + backend recall |
| reflection recall + enqueue | 支持 | backend recall/jobs + 本地 prompt 规划 |
| reflection enqueue source | 支持 | backend 持有的 transcript-backed source resolution |
| distill job enqueue + 轮询 | 支持 | backend 权威异步 job 面 |
| distill inline-messages 清洗 + artifact 持久化 | 支持 | backend 权威执行路径 |
| distill `session-transcript` source | 支持 | backend 持久化 transcript + 异步 distill 执行 |
| `memory_store` / `memory_update` / `memory_forget` | 支持 | 远程后端工具 |
| `memory_list` / `memory_stats` | 支持 | 可选管理工具 |
| `memory_reflection_status` | 支持 | 可选管理工具，用于 caller-scoped 的 backend reflection job |
| `memory_distill_enqueue` / `memory_distill_status` | 支持 | 可选管理工具，用于 caller-scoped 的 backend distill job |
| `memory_recall_debug` | 支持 | 可选管理/debug 工具，用于显式 recall trace 检查 |
| 本地 `memory-pro` CLI | 不支持 | 已移除 |
| 受支持的本地权威运行时 | 不支持 | 已移除 |

## 9. Distill：旧 sidecar 与当前 backend-native 方向

| 事项 | 历史 `jsonl_distill.py` / sidecar 流水线 | 当前 backend-native 方向 |
|---|---|---|
| Job ownership | 外部脚本 + worker | Rust backend job surface |
| Source preprocessing | 脚本本地过滤 / 清洗 | backend cleanup / filtering pipeline |
| Persistence | 外部再导回存储 | backend 自己持久化 artifacts，并可选写 memory |
| 状态检查 | 队列文件 / worker 日志 | `GET /v1/distill/jobs/{jobId}` |
| Runtime authority | 已不是 canonical path | backend-native 才是 canonical direction |

重要边界：

- 旧 `jsonl_distill.py` / example-worker sidecar 路径已经从活动 repo 运行时中移除
- 它不是当前受支持运行路径
- 当前受支持方向是 backend-native distill jobs，并由 backend 自己持久化 session transcript

## 10. 调试与可观测性

现在有两层可观测面：

| 接口 | 用途 | 契约稳定性 |
|---|---|---|
| `/v1/recall/*` | 正常运行时 recall | 稳定数据面 DTO |
| `/v1/debug/recall/*` | 检查 retrieval trace | 显式 debug 面，与普通 DTO 分离 |

重要边界：

- 普通 recall DTO 不暴露厚重 score-breakdown 内部字段
- debug trace route 的存在，是为了在不污染运行时契约的前提下补足调试能力
- `memory_recall_debug` 是这些 debug route 对应的 management-gated tool 面

## 11. 安装

### 克隆到 OpenClaw 插件目录

```bash
git clone https://github.com/furedericca-lab/openclaw-chronicle-engine.git ~/.openclaw/workspace/plugins/openclaw-chronicle-engine
cd ~/.openclaw/workspace/plugins/openclaw-chronicle-engine
npm install
```

### 绑定到 `memory` slot

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

### 验证

```bash
openclaw gateway restart
openclaw plugins info openclaw-chronicle-engine
openclaw config get plugins.slots.memory
```

## 12. 最小可用配置

把下面内容放到 `plugins.entries.openclaw-chronicle-engine.config`。

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

字段说明：

| 字段 | 必需 | 说明 |
|---|---|---|
| `remoteBackend.enabled` | 是 | 必须为 `true` |
| `remoteBackend.baseURL` | 是 | backend 地址 |
| `remoteBackend.authToken` | 是 | 运行时 bearer token |
| `timeoutMs` | 否 | 请求超时 |
| `maxRetries` | 否 | 传输层重试次数 |
| `retryBackoffMs` | 否 | 重试回退时间 |

兼容性说明：

- `sessionMemory.enabled` 仍映射到 `sessionStrategy`
- `sessionMemory.messageCount` 仍映射到 `memoryReflection.messageCount`
- `memoryReflection.agentId`
- `memoryReflection.maxInputChars`
- `memoryReflection.timeoutMs`
- `memoryReflection.thinkLevel`

`sessionMemory.*` 映射仍然只用于迁移兼容。上面列出的 `memoryReflection.*` 字段则是“可解析但忽略”的兼容字段：配置时会触发启动告警，在 remote-backend runtime 下不会改变 reflection 执行行为。

## 13. 工具

### 核心工具

- `memory_recall`
- `memory_store`
- `memory_forget`
- `memory_update`
- `self_improvement_log`

### 可选管理工具

启用 `enableManagementTools: true` 后可用：

- `memory_list`
- `memory_stats`
- `memory_reflection_status`
- `memory_distill_enqueue`
- `memory_distill_status`
- `memory_recall_debug`
- `self_improvement_review`
- `self_improvement_extract_skill`

这些 management/debug 工具仍然受 caller scope 和运行时主体身份约束，不提供匿名本地 fallback。

### Backend client 管理/调试面

插件侧 backend client 还提供：

- reflection source loading
- reflection jobs
- distill jobs
- recall debug traces

## 14. 仓库结构

```text
backend/                  Rust backend 实现
docs/runtime-architecture.md
docs/archive/             历史计划与已关闭 scope
src/backend-client/*      传输 + DTO 适配
src/backend-tools.ts      tool bridge
src/context/*             prompt-time orchestration
src/query-expander.ts     仅保留为 test/reference 词汇 helper
src/reflection-store.ts   仅保留为 test/reference reflection helper
test/*                    插件侧测试
```

## 15. 测试

### 插件测试

```bash
npm test
```

### Backend 测试

```bash
cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture
```

## 16. 常见误解

### “这还是本地 LanceDB 插件吗？”

不是。当前受支持模型是 remote authority only。

### “`src/context/*` 还在，说明本地权威没清干净？”

不是。`src/context/*` 只负责 prompt-time 编排：

- 什么时候 recall
- 怎么渲染注入 block
- 同一个 session 里怎么避免重复暴露

它不负责 backend 权威。

### “`setwise-v2` 是不是旧 TS RAG 没迁完？”

不是。它现在被定义成 `prompt-local seam`：

- 只对 backend 已经返回的普通 rows 做 lexical/coverage 导向的 prompt 注入层二次裁剪
- 不改变 backend authority
- 不重建 backend retrieval / rerank / embedding authority

### “`memoryReflection.agentId` / `maxInputChars` / `timeoutMs` / `thinkLevel` 现在还会控制 reflection 执行吗？”

不会。它们现在只是兼容字段，仍可解析，但在当前受支持运行时里会被忽略。真正生效的路径只有 backend reflection enqueue。

### “`src/query-expander.ts` 和 `src/reflection-store.ts` 还是运行时权威模块吗？”

不是。它们现在只保留为 test/reference helper，当前受支持运行时不会导入它们。

### “distill 现在是不是还靠旧的 `jsonl_distill.py` sidecar？”

不是。这个 sidecar 路径已经从当前活动运行时和 repo 中移除。

当前受支持方向是：

- backend-native distill jobs
- backend-owned status
- backend-owned artifacts
- backend-owned session transcript persistence 与 replay-safe source resolution

旧 sidecar / example 流水线不是 canonical runtime path。

## 17. 参考

- 运行时架构：`docs/runtime-architecture.md`
- 文档索引：`docs/README.md`
- 历史执行与关闭 scope：`docs/archive/`
- 插件 schema：`openclaw.plugin.json`

## License

MIT
