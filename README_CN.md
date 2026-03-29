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
- 面向 backend 的 recall 过滤语义也留在 backend；插件只保留 prompt-time 编排和渲染

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
| behavioral-guidance recall 检索 | 负责 | 只负责 prompt-time autoRecall guidance 注入规划 |
| distill job 执行 | 负责 | 只负责 enqueue / poll |
| distill source 清洗与 artifact 持久化 | 负责 | 不负责 |
| distill lesson / governance derivation | 负责 | 不负责 |
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

### cadence 驱动的 distill 流

```text
agent_end
  -> 插件追加有序 transcript rows
    -> backend 持久化 session transcript
      -> 每累计 distill.everyTurns 个 user turn
        -> 插件发起 POST /v1/distill/jobs
          -> backend 从 session trajectory 提炼 distill artifacts
            -> 后续 recall / injection 再读取已持久化 rows 与 artifacts
```

### distill job 流

```text
distill 请求
  -> 插件 / backend client
    -> POST /v1/distill/jobs
        -> backend 校验 actor + source + mode
        -> backend 异步排队 distill job
          -> backend 清洗 transcript / messages 并构建 deterministic span/window candidates
            -> backend 持久化英文 distill artifacts
              -> 可选写入 memory rows
                -> GET /v1/distill/jobs/{jobId} 查看状态和结果
```

## 4.1 Admin Plane 与 token 设置

Chronicle Engine 带有内置的管理后台，入口是 `/admin`。

- `auth.runtime.token` 只用于 `/v1/*` 数据面请求
- `auth.admin.token` 只用于 `/admin/api/*` 管理面请求
- 两者都使用 `Authorization: Bearer <token>`
- 两套 token 不能混用

它们在 `backend.toml` 中这样配置：

```toml
[auth.runtime]
token = "replace-with-runtime-bearer-token"

[auth.admin]
token = "replace-with-admin-bearer-token"
```

对于 Docker 部署，现在镜像内部会自带默认 `backend.toml`。推荐做法是不再默认挂载配置文件，而是在 Docker Compose 的 `environment:` 中用 `CHRONICLE_` 前缀覆盖对应配置项，嵌套 TOML 路径用双下划线表示，例如：

```yaml
environment:
  CHRONICLE_AUTH__RUNTIME__TOKEN: "${CHRONICLE_AUTH_RUNTIME_TOKEN}"
  CHRONICLE_AUTH__ADMIN__TOKEN: "${CHRONICLE_AUTH_ADMIN_TOKEN}"
  CHRONICLE_PROVIDERS__EMBEDDING__API_KEY: "${CHRONICLE_EMBEDDING_API_KEY}"
```

## 4.2 部署后最短验证命令

```bash
cd /root/.openclaw/workspace/plugins/openclaw-chronicle-engine

git rev-parse --short HEAD
docker compose -f deploy/docker-compose.yml config >/dev/null

curl -fsS http://127.0.0.1:8080/admin >/dev/null

curl -fsS \
  -H "Authorization: Bearer $CHRONICLE_ADMIN_TOKEN" \
  http://127.0.0.1:8080/admin/api/settings/runtime-config

curl -fsS \
  -H "Authorization: Bearer $CHRONICLE_RUNTIME_TOKEN" \
  -H "X-OpenClaw-User-Id: smoke-user" \
  -H "X-OpenClaw-Agent-Id: smoke-agent" \
  -H "Content-Type: application/json" \
  -d '{"query":"smoke check","topK":3}' \
  http://127.0.0.1:8080/v1/recall/generic

curl -s -o /dev/null -w "%{http_code}\n" \
  -H "Authorization: Bearer $CHRONICLE_ADMIN_TOKEN" \
  -H "X-OpenClaw-User-Id: smoke-user" \
  -H "X-OpenClaw-Agent-Id: smoke-agent" \
  -H "Content-Type: application/json" \
  -d '{"query":"should fail","topK":1}' \
  http://127.0.0.1:8080/v1/recall/generic

curl -s -o /dev/null -w "%{http_code}\n" \
  -H "Authorization: Bearer $CHRONICLE_RUNTIME_TOKEN" \
  http://127.0.0.1:8080/admin/api/settings/runtime-config
```

预期：

- `git rev-parse` 输出当前部署提交
- `/admin` 能返回前端页面
- admin token 能访问 `/admin/api/settings/runtime-config`
- runtime token 能访问 `/v1/recall/generic`
- admin token 不能成功访问 `/v1/*`
- runtime token 不能成功访问 `/admin/api/*`

## 4.3 Distill 和 Settings 扩展验证

Settings 当前语义是“在线保存并原子写盘，但仍需重启生效”。

```bash
curl -fsS \
  -X PUT \
  -H "Authorization: Bearer $CHRONICLE_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d @- \
  http://127.0.0.1:8080/admin/api/settings/runtime-config <<'JSON'
{"configToml":"[server]\nbind = \"0.0.0.0:8080\"\nadmin_assets_path = \"/usr/local/bin/web/dist\"\n\n[storage]\nlancedb_path = \"/var/lib/chronicle-engine-backend/lancedb\"\nsqlite_path = \"/var/lib/chronicle-engine-backend/sqlite/jobs.db\"\n\n[auth.runtime]\ntoken = \"replace-with-runtime-bearer-token\"\n\n[auth.admin]\ntoken = \"replace-with-admin-bearer-token\"\n\n[logging]\nlevel = \"info\"\n\n[providers.embedding]\nbase_url = \"https://api.openai.com/v1\"\nmodel = \"text-embedding-3-small\"\napi = \"openai\"\napi_key = \"replace-with-embedding-api-key\"\n\n[providers.rerank]\nenabled = false\n"}
JSON

curl -fsS \
  -H "Authorization: Bearer $CHRONICLE_RUNTIME_TOKEN" \
  -H "X-OpenClaw-User-Id: smoke-user" \
  -H "X-OpenClaw-Agent-Id: smoke-agent" \
  -H "Content-Type: application/json" \
  -d '{"source":{"type":"inline_messages","messages":[{"role":"user","content":"I prefer concise release checklists."},{"role":"assistant","content":"Keep release verification short and explicit."}]},"mode":"session_lessons","persistMemoryRows":true}' \
  http://127.0.0.1:8080/v1/distill/jobs
```

预期：

- Settings 保存成功，并返回 `restartRequired: true` 或 `restart_required: true`
- distill enqueue 返回 job id
- 后续可继续调用 `GET /v1/distill/jobs/{jobId}` 查看状态

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
| behavioral-guidance recall 权威 | 本地 TS + 本地持久化路径 | Rust backend recall 路径 + 插件侧 autoRecall behavioral 渲染 | 已替换 |
| 命令触发的 trajectory-derived generation | 本地 / 插件耦合执行 | 已移除；当前只保留 cadence 驱动的 distill 生成 | 已移除 |
| Distill async jobs | 历史 sidecar / example 流水线 | Rust backend distill jobs | 已是 backend-native deterministic runtime |
| Scope derivation / ACL | 历史上本地 TS 有参与 | 仅 Rust backend | 已替换 |
| 可检查 retrieval trace | 历史 TS 有更厚 telemetry 对象 | Rust backend debug trace routes | 达到可接受 parity，但不是 1:1 复刻 |
| Prompt 注入渲染 | 本地 TS | 本地 TS | 有意保留 |
| Session-local 暴露抑制 | 本地 TS | 本地 TS | 有意保留 |
| 通用 auto-recall 最终裁剪 | 本地 TS | 本地 TS，作用于 backend 已返回 rows | 仅限直接 prompt 注入截断 |

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
| `src/context/recall-engine.ts` | 本地 gating / dedupe / exposure-state helper |
| `src/context/adaptive-retrieval.ts` | prompt 侧 recall 触发启发式 |

### 实际应该怎么理解

如果问题是：

- “旧 TS authority 链路还活着吗？” -> **没有**
- “仓库里还有没有与 recall/behavioral guidance 相关的 TS 文件？” -> **有，而且是有意保留，用于 prompt-local 编排和测试**

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
| Distill enqueue | 明确失败，fail-closed |

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
| autoRecall behavioral-guidance planning | 支持 | behavioral recall 只读检索在 backend，prompt guidance 注入规划在插件 |
| distill job enqueue + 轮询 | 支持 | backend 权威异步 job 面 |
| distill inline-messages 清洗 + artifact 持久化 | 支持 | backend 权威执行路径 |
| distill `session-transcript` source | 支持 | backend 持久化 transcript + 异步 distill 执行 |
| 每 N 个 user turn 自动 distill | 支持 | runtime cadence + backend-native `session-transcript` jobs |
| `session-lessons` 模式 | 支持 | 负责 lesson、cause、fix、prevention、stable decision、durable practice，并对 stable decision / durable practice 使用 evidence gate 升格 |
| `governance-candidates` 模式 | 支持 | 负责值得提升的 learnings、skill extraction candidates、AGENTS/SOUL/TOOLS promotion candidates |
| distill artifact 子类型 | 支持 | `follow-up-focus` 和 `next-turn-guidance` 取代独立 derived/open-loop reflection 持久化 |
| `memory_store` / `memory_update` / `memory_forget` | 支持 | 远程后端工具 |
| `memory_list` / `memory_stats` | 支持 | 可选管理工具 |
| `memory_distill_enqueue` / `memory_distill_status` | 支持 | 可选管理工具，用于 caller-scoped 的 backend distill job |
| `memory_recall_debug` | 支持 | 可选管理/debug 工具，用于显式 recall trace 检查 |
| 本地 `memory-pro` CLI | 不支持 | 已移除 |
| 受支持的本地权威运行时 | 不支持 | 已移除 |

## 9. Backend 功能总览

如果只用最短的话概括 backend 现在负责什么，可以直接看下面：

- 它负责所有 durable memory 的写入、更新、删除、list/stats 读取，以及 recall 检索
- 它负责 ranking、rerank fallback、MMR、time-decay、access reinforcement，以及 scope / ACL enforcement
- 它负责 behavioral-guidance recall rows，以及 caller-scoped 的 debug recall trace 接口
- 它负责 transcript persistence 和 async distill jobs，包括 artifact 持久化，以及可选的 distilled memory-row persistence

当前对外真正重要的 backend route family 是：

| 路由族 | 用途 |
|---|---|
| `/v1/memories/store`、`/update`、`/delete`、`/list`、`/stats` | caller-scoped 的 memory 写入/读取管理 |
| `/v1/recall/generic` | 正常运行时 recall |
| `/v1/recall/behavioral` | backend-managed 的 behavioral-guidance recall |
| `/v1/debug/recall/generic`、`/behavioral` | 显式 retrieval trace 检查 |
| `/v1/session-transcripts/append` | 持久化有序 runtime transcript rows |
| `/v1/distill/jobs` 与 `/v1/distill/jobs/{jobId}` | enqueue 并检查异步 distill job |

有两个概念很容易混淆，但当前是明确分开的：

- `behavioral recall`：读取 backend-managed behavioral rows，用于 prompt-time guidance 注入
- `distill`：从 transcript 或 inline messages 里提炼 artifact，并可选继续落成 memory rows

## 10. 插件明确不负责什么

插件侧还有不少运行时代码，但它不能成为第二套 authority。

插件不负责：

- durable persistence
- 权威 recall / ranking / rerank 决策
- scope derivation 或 ACL visibility 决策
- backend-facing recall filter semantics
- 独立的 sidecar / queue-file distill 流水线

插件负责：

- OpenClaw hook / tool 集成
- backend transport / DTO adapter
- prompt-time planning、rendering，以及 session-local exposure suppression

## 11. Distill：旧 sidecar 与当前 backend-native 方向

| 事项 | 历史 `jsonl_distill.py` / sidecar 流水线 | 当前 backend-native 方向 |
|---|---|---|
| Job ownership | 外部脚本 + worker | Rust backend job surface |
| Source preprocessing | 脚本本地过滤 / 清洗 | backend cleanup / filtering pipeline |
| Reduction quality | sidecar reduction pipeline | deterministic Rust turns-stage lesson reducer |
| Persistence | 外部再导回存储 | backend 自己持久化 artifacts，并可选写 memory |
| 状态检查 | 队列文件 / worker 日志 | `GET /v1/distill/jobs/{jobId}` |
| Runtime authority | 已不是 canonical path | backend-native 才是 canonical direction |

当前运行时形态：

- runtime 在 `agent_end` 时把有序 transcript rows append 到 backend
- runtime 可按配置的 `distill.everyTurns`，每累计 N 个 user turn 自动 enqueue 一个 backend-native `session-transcript` distill job
- backend 自己解析 source rows、做清洗、构建 deterministic span/window candidates、聚合 evidence，并持久化 artifacts
- 当 `persistMode=persist-memory-rows` 时，backend 还会把最终 artifact 继续落成 memory rows

当前行为边界：

- 旧 `jsonl_distill.py` / example-worker sidecar 路径已经从活动 repo 运行时中移除
- 它不是当前受支持运行路径
- 当前受支持方向是 backend-native distill jobs，并由 backend 自己持久化 session transcript
- 当前 distill summary 刻意保持为英文 deterministic 输出
- 可选 runtime cadence 会按 `distill.everyTurns` 自动 enqueue 一个 `session-transcript` distill job

当前 distill 擅长的事：

- 不依赖 sidecar 基础设施的 deterministic turns-stage lesson extraction
- 在 backend reduction window 内做多消息 evidence 聚合
- 在同一 caller-scoped backend authority 模型下稳定产出 artifact，并可选继续持久化 memory
- 把所有新学习写入统一收敛到 `session-lessons` 和 `governance-candidates`

当前 distill 明确不做的事：

- 语言自适应抽取
- 单独的非 distill 生成流水线
- 恢复 queue-file / worker / `memory-pro import` 这一套旧架构

## 12. 调试与可观测性

现在有两层可观测面：

| 接口 | 用途 | 契约稳定性 |
|---|---|---|
| `/v1/recall/*` | 正常运行时 recall | 稳定数据面 DTO |
| `/v1/debug/recall/*` | 检查 retrieval trace | 显式 debug 面，与普通 DTO 分离 |

重要边界：

- 普通 recall DTO 不暴露厚重 score-breakdown 内部字段
- debug trace route 的存在，是为了在不污染运行时契约的前提下补足调试能力
- `memory_recall_debug` 是这些 debug route 对应的 management-gated tool 面

## 13. 安装

### 克隆到 OpenClaw 插件目录

```bash
git clone https://github.com/furedericca-lab/chronicle-engine.git ~/.openclaw/workspace/plugins/chronicle-engine
cd ~/.openclaw/workspace/plugins/chronicle-engine
npm install
```

### 绑定到 `memory` slot

```json
{
  "plugins": {
    "load": {
      "paths": ["plugins/chronicle-engine"]
    },
    "entries": {
      "chronicle-engine": {
        "enabled": true,
        "config": {}
      }
    },
    "slots": {
      "memory": "chronicle-engine"
    }
  }
}
```

### 验证

```bash
openclaw gateway restart
openclaw plugins info chronicle-engine
openclaw config get plugins.slots.memory
```

## 14. 最小可用配置

把下面内容放到 `plugins.entries.chronicle-engine.config`。

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

cutover 说明：

- `1.0.0-beta.0` 已移除只为迁移保留的 config alias。
- 只支持 `sessionStrategy: "autoRecall" | "systemSessionMemory" | "none"`。
- `autoRecallBehavioral.*` 是当前行为指导配置的 canonical surface。
- backlog/review 工作流配置请使用 `governance.*`。
- 历史迁移别名配置都会被直接拒绝。

## 15. 工具

### 核心工具

- `memory_recall`
- `memory_store`
- `memory_forget`
- `memory_update`
- `governance_log`

### 可选管理工具

启用 `enableManagementTools: true` 后可用：

- `memory_list`
- `memory_stats`
- `memory_distill_enqueue`
- `memory_distill_status`
- `memory_recall_debug`
- `governance_review`
- `governance_extract_skill`

这些 management/debug 工具仍然受 caller scope 和运行时主体身份约束，不提供匿名本地 fallback。

### Backend client 管理/调试面

插件侧 backend client 还提供：

- distill jobs
- recall debug traces（`generic` 与 `behavioral`）

## 16. 仓库结构

```text
backend/                  Rust backend 实现
docs/runtime-architecture.md
docs/archive/             历史计划与已关闭 scope
src/backend-client/*      传输 + DTO 适配
src/backend-tools.ts      tool bridge
src/context/*             prompt-time orchestration
test/*                    插件侧测试
```

## 17. 测试

### 插件测试

```bash
npm test
```

### Backend 测试

```bash
cargo test --manifest-path backend/Cargo.toml --test contract_semantics -- --nocapture
```

## 18. 常见误解

### “这还是本地 LanceDB 插件吗？”

不是。当前受支持模型是 remote authority only。

### “`src/context/*` 还在，说明本地权威没清干净？”

不是。`src/context/*` 只负责 prompt-time 编排：

- 什么时候 recall
- 怎么渲染注入 block
- 同一个 session 里怎么避免重复暴露

它不负责 backend 权威。

### “旧的配置别名现在还能用吗？”

不能。请只使用当前 schema 中的名字：

- `sessionStrategy`
- `autoRecallBehavioral.*`
- `governance.*`

### “distill 现在是不是还靠旧的 `jsonl_distill.py` sidecar？”

不是。这个 sidecar 路径已经从当前活动运行时和 repo 中移除。

当前受支持方向是：

- backend-native distill jobs
- backend-owned status
- backend-owned artifacts
- backend-owned session transcript persistence 与 replay-safe source resolution

旧 sidecar / example 流水线不是 canonical runtime path。

## 19. 参考

- 运行时架构：`docs/runtime-architecture.md`
- 文档索引：`docs/README.md`
- 历史执行与关闭 scope：`docs/archive/`
- 插件 schema：`openclaw.plugin.json`

## License

MIT
