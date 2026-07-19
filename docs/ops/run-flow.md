# anyCode 运行流程总览

面向维护者与高级用户：从进程启动到 Agent 执行、工作台观测的完整链路。

**相关文档**

- 分层与扩展点：[`architecture.md`](../architecture.md)
- 用户向工作台说明：[`https://anycode.work/docs/guide/workbench.md`](../docs/user/guide/workbench)
- ADR 000（编排权威）：[`adr/000-agent-runtime-orchestration.md`](adr/000-agent-runtime-orchestration.md)

## 核心结论

1. **三个入口**：**anyCode.app**（内嵌 dashboard）、**`anycode-daemon`**（通道 + scheduler）、以及开发时直接跑 dashboard/desktop crate。
2. **Agent 执行内嵌**：Desktop 与 dashboard HTTP 服务在同一进程内通过 **`anycode-bootstrap::initialize_runtime`** 构造 `AgentRuntime`；**不再** embedded AgentRuntime 子进程。
3. **配置统一**：`~/.anycode/config.json`（`anycode-config`）；首次配置走 Workbench **`/setup`**。

## 进程拓扑

```text
┌─────────────────────────────────────────────────────────────────┐
│  anyCode Desktop（apps/anycode-desktop）                         │
│    进程内启动 anycode-dashboard HTTP (:43180)                    │
│    WebView → http://127.0.0.1:43180/                           │
│    可选：用户自行运行 anycode-daemon wechat-bridge 等            │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│  anycode-dashboard（Axum，crates/dashboard）                     │
│    SQLite: ~/.anycode/projects.db                               │
│    静态 UI: dashboard-ui/dist / embedded-ui                     │
│    Web 聊天 / UI trigger → 内嵌 AgentRuntime（bootstrap）        │
│    审批/取消: dashboard-ipc 文件 IPC                           │
└────────────────────────────┬────────────────────────────────────┘
                             │ in-process
┌────────────────────────────▼────────────────────────────────────┐
│  anycode-bootstrap → AgentRuntime → tools / LLM / memory        │
│    录制: DashboardRecorder → projects.db                        │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  anycode-daemon（crates/channel-bridge 二进制）                  │
│    scheduler | wechat-bridge | telegram-bridge | discord-bridge │
│    同样 initialize_runtime；与 Desktop 共享 config.json          │
└─────────────────────────────────────────────────────────────────┘
```

## 入口对照

| 场景 | 入口 | AgentRuntime |
|------|------|----------------|
| macOS 日常使用 | anyCode.app | 内嵌（dashboard 库） |
| Workbench 对话 | HTTP `/api/...` chat / trigger | 内嵌 |
| 微信/Telegram/Discord | `anycode-daemon *-bridge` | 桥进程内 |
| Cron / 自动化 | `anycode-daemon scheduler` 或 Desktop 内嵌 | 调度循环内 |
| 开发调试 | `cargo tauri dev` / dashboard e2e server | 内嵌 |

## UI 触发任务（简化）

1. 用户在 Workbench 点击触发或发送消息  
2. `web_chat` / `task_trigger` 调用内嵌 runtime（`execute_task` / `execute_turn_from_messages`）  
3. 流式事件经 SSE 推送到前端；可选写入 `projects.db`

## 审批路径

- **Workbench**：Settings → Security；进行中审批走 Web inbox + `dashboard-ipc`  
- **通道**：微信/Telegram/Discord 桥内 headless 或交互审批回调  

## 已移除

- 终端 `anycode` 二进制（REPL/TUI/`run`/`setup`/`dashboard` 子命令）  
- HTTP `anycode daemon`（POST `/v1/tasks`）— 见 ADR 003  
- Dashboard spawn CLI 子进程执行 Agent  

## 代码锚点

| 区域 | 路径 |
|------|------|
| Desktop 内嵌 dashboard | `apps/anycode-desktop/src/dashboard_backend.rs` |
| Runtime 组装 | `crates/bootstrap/src/runtime.rs` |
| Web 聊天 | `crates/dashboard/src/control/web_chat.rs` |
| UI trigger | `crates/dashboard/src/control/`（embedded path） |
| Daemon 入口 | `crates/channel-bridge/src/bin/anycode_daemon.rs` |
| 通道桥 | `crates/channel-bridge/src/channels/` |
