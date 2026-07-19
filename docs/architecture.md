# anyCode 架构说明

面向维护者：分层、依赖方向与扩展点，避免「为抽象而抽象」。

**文档站**（中英）：[`https://anycode.work/docs/guide/architecture.md`](../docs/user/guide/architecture)；扩展清单 [`contributing-extensions.md`](../https://anycode.work/docs/guide/contributing-extensions)。**ADR** 在 [`docs/adr/`](adr/)。运行流程见 [`ops/run-flow.md`](ops/run-flow.md)。

## 分层与数据流

```text
anycode-desktop / anycode-daemon   ← 产品入口
    ↓
anycode-bootstrap                ← initialize_runtime 组合根
    ↓
anycode-agent                    ← AgentRuntime
    ↓
anycode-core                     ← 领域类型 + trait
    ↑
anycode-tools / llm / security / memory
```

**依赖规则**

- `core` 不依赖 agent / bootstrap / tools。
- `agent` 编排多轮循环；不实现具体工具。
- `bootstrap` 构造 `AgentRuntime`；Desktop、dashboard 内嵌聊天、daemon 桥接共用。

## 扩展点（优先使用顺序）

1. **新工具**：`anycode-tools` `registry.rs` + `catalog`。
2. **新 LLM 提供商**：`anycode-llm`。
3. **新 Agent 类型**：`Agent` trait + `register_agent`。

## Crate 要点

| Crate | 职责 |
|--------|------|
| `bootstrap` | `initialize_runtime`、工具/安全/记忆组装 |
| `dashboard` | Workbench HTTP、SQLite、内嵌 Agent 执行 |
| `channel-bridge` | `anycode-daemon` 二进制 |
| `agent` | `AgentRuntime`、`execute_task` / `execute_turn_from_messages` |
| `core` | `Message` / `Task`、trait |
| `tools` | 工具实现与注册表 |

## 设计原则

- 请求从 Workbench 或 daemon 进入 `AgentRuntime` 后，在 agent crate 内完成 LLM + 工具循环。
- 子模块拆分优先于新抽象。

## 已移除

- 终端 `anycode` CLI（REPL/TUI/`run`/`setup`/`dashboard` 子命令）
- HTTP `anycode daemon`（ADR 003）
- Dashboard spawn CLI 子进程

## 微信桥

见 [`wx-streaming-bridge.md`](ops/wx-streaming-bridge.md)。编排路径为 `execute_task`；daemon：`anycode-daemon wechat-bridge`。

## 定时任务（Cron）

- 工具：`CronCreate` / `CronDelete` / `CronList` → `~/.anycode/tasks/orchestration.json`
- 执行：`anycode-daemon scheduler`（`crates/channel-bridge/src/scheduler.rs`）

## Digital Workbench（Dashboard）与 Desktop

**单一 `anycode` 二进制、多运行模式**（详见 [`run-flow.md`](ops/run-flow.md)）：

- **`anyCode Workbench`**：Axum HTTP 服务（默认 `127.0.0.1:43180`），SQLite `~/.anycode/projects.db`，嵌入或挂载 `dashboard-ui` 静态资源。
- **Agent 不在 dashboard 进程内执行**：UI 通过 `task_trigger` embedded AgentRuntime 子进程；`DashboardRecorder` tail `output.log` 写入 DB；审批/取消经文件 IPC（`approval_ipc` / `cancel_ipc`）。
- **Desktop（Tauri）**：`apps/anycode-desktop` 仅 spawn `anyCode Workbench` sidecar + WebView，不直接调用 `initialize_runtime`。
- **Project**：工作台「项目」= 磁盘工作区 + DB 元数据；模板（`project-templates/`）、Gate Runner、知识库索引见 `crates/dashboard/` 与 `crates/tools/src/project_templates/`。

关键 crate：`crates/dashboard`（HTTP API + 录制）、`crates/dashboard-ui`（React 工作台 UI）。
- **单实例**：**`~/.anycode/tasks/scheduler.lock`**（独占锁）保证同机只有一个调度循环。**WeChat / Telegram / Discord** 长驻桥与独立 **`anycode-daemon scheduler`** **抢同一把锁**；任一桥在启动时 **`tokio::spawn`** 内置调度器的尝试抢锁失败时静默退出嵌入（聊天仍可用，但须有**另一进程**持锁才能使 cron 落火）。
- **通道 agent**：`workspace-assistant` 暴露上述 Cron 工具以便 IM 侧创建/删除任务；用户文档见文档站 [Cron & scheduler](../https://anycode.work/docs/guide/cli-scheduler)。

迭代任务与决策状态见 **[`docs/roadmap.md`](roadmap.md)**（SSOT）；MVP 与工具矩阵见文档站 [Roadmap](../https://anycode.work/docs/guide/roadmap）。
