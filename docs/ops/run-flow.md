# anyCode 运行流程总览

面向维护者与高级用户：从进程启动到 Agent 执行、工作台观测的完整链路。

**相关文档**

- 分层与扩展点：[`architecture.md`](../architecture.md)
- 用户向工作台说明：[`docs-site/guide/workbench.md`](../docs-site/guide/workbench.md)
- ADR 000（编排权威）：[`adr/000-agent-runtime-orchestration.md`](adr/000-agent-runtime-orchestration.md)

## 核心结论

1. **单一二进制**：`anycode` 既是 CLI（TTY / REPL / run / channel），也是 Dashboard HTTP 服务（`anycode dashboard`）。
2. **Agent 执行只在 CLI 子进程**：Dashboard 与 Desktop 不内嵌 `AgentRuntime`；UI 触发任务时 spawn `anycode run`。
3. **Desktop 是壳**：Tauri 进程 spawn `anycode dashboard` sidecar + WebView，所有能力仍来自 CLI 二进制。

## 进程拓扑

```text
┌─────────────────────────────────────────────────────────────────┐
│  Tauri Desktop（apps/anycode-desktop）                           │
│    spawn → anycode dashboard  (:43180)                           │
│    WebView → http://127.0.0.1:43180/                           │
│    可选 spawn → anycode channel wechat --run-as-bridge          │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│  anycode dashboard（Axum HTTP，crates/dashboard）                │
│    SQLite: ~/.anycode/projects.db                               │
│    静态 UI: dashboard-ui/dist 或 embedded-ui feature            │
│    POST trigger → spawn anycode run -C <root> ...               │
│    审批/取消: approval_ipc / cancel_ipc 文件 IPC                 │
└────────────────────────────┬────────────────────────────────────┘
                             │ spawn
┌────────────────────────────▼────────────────────────────────────┐
│  anycode run / REPL / channel（同一二进制）                      │
│    initialize_runtime → AgentRuntime                            │
│    tail output.log → DashboardRecorder → projects.db            │
└─────────────────────────────────────────────────────────────────┘
```

## 入口与模式

| 模式 | 启动方式 | Agent 路径 | Dashboard 联动 |
|------|----------|------------|----------------|
| Stream TTY（默认） | 无子命令 + TTY | `execute_turn_from_messages` | `DashboardRecorder` + Web 审批 IPC |
| `anycode repl` | 显式子命令 | 同上 | 同上 |
| `anycode run` | 子命令 / UI 触发 | `execute_task` | `ANYCODE_DASHBOARD_RECORD=1` 时录制 |
| Channel（微信等） | `anycode channel …` | `execute_task` | 可选读 project skills |
| `anycode dashboard` | 子命令 / Desktop sidecar | **不跑 AgentRuntime** | 自身即 HTTP 服务 |

入口分发见 `crates/cli/src/commands/dispatch/mod.rs`；组合根见 `crates/cli/src/bootstrap/runtime.rs::initialize_runtime`。

## 数据流（一次 UI 触发的任务）

```text
1. 用户在 Dashboard 输入 prompt
2. task_trigger  spawn: anycode run -C <project_root> --prompt "…"
3. CLI initialize_runtime → AgentRuntime::execute_task
4. 执行过程写 ~/.anycode/tasks/<id>/output.log（结构化 trace 行）
5. DashboardRecorder tail output.log → INSERT project_events / sessions
6. SSE 推送 → dashboard-ui 刷新 EventTimeline / ConversationTranscript
7. 敏感工具需审批 → Web UI respond → approval_ipc → CLI WorkbenchApprovalCallback
8. 用户取消 → cancel_ipc → CooperativeCancel（ADR 010）
```

关键文件：

- 触发：`crates/dashboard/src/control/task_trigger.rs`
- 录制：`crates/dashboard/src/recorder.rs`
- 日志解析：`crates/dashboard/src/observability/log_parser.rs`
- 对话转录：`crates/dashboard/src/observability/session_transcript.rs`

## 组合根组装（initialize_runtime）

| 步骤 | 模块 | 产物 |
|------|------|------|
| LLM | `build_llm_stack` | `Arc<dyn LLMClient>` |
| 记忆 | `build_memory_layer` | MemoryStore + 可选 MemoryPipeline |
| 安全 | `build_security_setup` | SecurityLayer、MCP defer gate |
| 工具 | `build_tools_setup` | 工具注册表、Skill catalog、MCP 连接 |
| Prompt | `prompt_runtime` | 工作区 / skills / project allowlist |
| 运行时 | `AgentRuntime::new` | 唯一多轮编排门面 |

编排权威仅在 `crates/agent/src/runtime/`（ADR 000）。

## Project（工作台项目）

**Project = 磁盘工作区 + SQLite 元数据**（`~/.anycode/projects.db`）。

| 阶段 | 说明 | 关键路径 |
|------|------|----------|
| 创建 | UI `POST /api/projects` 或 `anycode project init` | `handlers/projects.rs`、`project_templates/` |
| 配置 | 知识库路径、Skills 白名单、Gate 预设 | `ProjectKnowledgeConfigPanel`、`gate_runner.rs` |
| 执行 | trigger → `anycode run` / goal | `task_trigger.rs` |
| 验证 | Gate Runner 在 project_root 跑 shell | `control/gate_runner.rs` |
| 观测 | sessions、events、reports | `recorder.rs`、`session_transcript.rs` |

## MCP 与 Skill

| 机制 | 配置入口 | 运行时 |
|------|----------|--------|
| **MCP** | `config.json` → `mcp.servers`；环境变量 `ANYCODE_MCP_SERVERS` 按 slug 覆盖 | `bootstrap/mcp_env.rs` → `tools_setup.rs` 长连接 |
| **MCP（UI）** | 设置 → 通知与连接器 → MCP 服务器（JSON 编辑） | `GET/PUT /api/settings/mcp-servers` |
| **内置浏览器** | `mcp.browser.enabled` + 桌面包 `ANYCODE_BROWSER_MCP_ROOT` | Playwright MCP（stdio，slug=`browser`） |
| **Skill** | `~/.anycode/skills`、项目 `skills/`；`config.json` → `skills.*` | `SkillCatalog` + `Skill` 工具 |

## 配置与共享状态

| 路径 | 用途 |
|------|------|
| `~/.anycode/config.json` | 全局配置（LLM、memory、skills、mcp、agents…） |
| `<workspace>/.anycode/config.json` | 工作区 overlay |
| `~/.anycode/projects.db` | Dashboard 项目 / 会话 / 事件 |
| `~/.anycode/tasks/<id>/output.log` | 任务执行 trace |
| `~/.anycode/sessions/` | REPL/TUI 会话快照 |

## Desktop 打包

`scripts/build-desktop-release.sh` 产出 macOS DMG；sidecar 内嵌：

- `anycode` 二进制
- `dashboard-ui/dist`（或 `embedded-ui`）
- `project-templates/`
- `resources/browser/`（Playwright MCP + Chromium，由 `prepare-browser-mcp.sh` 生成）

Tauri 启动逻辑：`apps/anycode-desktop/src/main.rs`。

### 构建时下载 vs 最终安装包

| 命令 | 浏览器 MCP / Chromium | dashboard-ui `npm ci` | 说明 |
|------|----------------------|----------------------|------|
| `cargo build --release -p anycode` | **否** | 仅当 `dist/` 缺失时 | 仅 CLI |
| `./scripts/build-desktop-release.sh` | **是**（首次或 lockfile 变更） | **是**（首次或 lockfile 变更） | 写入 `resources/browser/` 后打进 DMG/App |

产物路径（与 CLI 共用根目录 `target/`）：`target/release/bundle/dmg/`、`target/release/bundle/macos/`。

**用户**安装 DMG 后无需再执行 `npx playwright install`。

**开发者重复打包**时的本地缓存（lockfile / 平台未变则跳过下载）：

| 缓存 | 位置 | 强制刷新 |
|------|------|----------|
| dashboard-ui npm | `crates/dashboard-ui/.npm-fingerprint` | `ANYCODE_DASHBOARD_UI_FORCE=1` |
| browser MCP + Chromium | `resources/browser/.bundle-fingerprint` | `ANYCODE_BROWSER_MCP_FORCE=1` |
| 桌面图标 | `icons/.icon-fingerprint` | `ANYCODE_DESKTOP_ICON_FORCE=1` |
| apple-media Swift | 源文件 mtime vs 已构建二进制 | `ANYCODE_APPLE_MEDIA_FORCE=1` |

`build-desktop-release.sh` 会打印每步耗时。若**每次**仍看到 `npm ci` 或 `playwright install`，检查是否设置了 `*_FORCE=1`、是否改了 lockfile、或是否删除了 `resources/browser/`。

dashboard-ui 已构建时可设 `ANYCODE_SKIP_DASHBOARD_UI_BUILD=1` 跳过 UI 的 npm build（`crates/dashboard/build.rs`）。

建议本地一次性安装 Tauri CLI，避免脚本内 `cargo install tauri-cli`：`cargo install tauri-cli --version "^2" --locked`。

Whisper / FastEmbed / Piper 等模型**不在 build 阶段打包**，首次使用时下载到 `~/.anycode` 或 `~/.cache`。
