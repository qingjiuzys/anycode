---
title: 路线图
description: MVP 范围、验收场景、工具阶段矩阵与 MCP/LSP 等后续里程碑。
summary: 发布边界、如何验收、P0–P8 与后续工作入口。
read_when:
  - 规划版本或对比 anyCode 与其它终端 Agent。
  - 开发 MCP、LSP、子 Agent 相关功能。
---

# 路线图

本文合并原 **MVP 范围**、**MVP 验收**、**工具与阶段（tools-parity）**、**待实现清单（roadmap-stubs）**、**MCP 后续（mcp-postmvp）** 的要点。**代码事实来源**仍以 `crates/tools/src/catalog.rs`、`crates/tools/src/agent_tools.rs`（**Agent** / **Task**）、`crates/cli/src/bootstrap/mod.rs`、`crates/agent/src/agents.rs` 为准。

**维护者执行层 backlog**（now / next / later、决策）：仓库 **[`docs/roadmap.md`](https://github.com/qingjiuzys/anycode/blob/main/docs/roadmap.md)** — 迭代任务请只改该文件，勿在本页重复清单。

## 最小 MVP 范围（已冻结）

**MVP 内**

- 单一工作区内 **读/写/改文件、Glob/Grep、Bash**（及文档已列的 P1/P2/P7/P8 内置工具）。  
- **审批 / 沙箱**（`SecurityLayer` 与配置项）。  
- **z.ai（OpenAI 兼容）与 Anthropic** 至少一条日常可用的 tool-calling 路径。  
- **执行期落盘日志**（`~/.anycode/tasks/<id>/output.log`）与 **结束 summary**（非 TUI 直出场景）。  
- **CLI**：`run`、`repl`、`tui`、通道桥、`scheduler` 等。（**HTTP `daemon`** 已移除 — 见 [ADR 003](https://github.com/qingjiuzys/anycode/blob/main/docs/adr/003-http-daemon-deprecated.md) 与 [HTTP 守护进程（已移除）](./cli-daemon)。）

**MVP 外**（独立里程碑，**不阻塞**上述 MVP 发布）

- **MCP** 完整产品形态（SSE/HTTP、完整 OAuth UI、延迟加载等）超出当前 stdio **v1** 范围的部分。  
- **LSP** 完整子进程故事（实验性 `tools-lsp` 之外）。  
- **子 Agent**：与上游 **完全**同级的隔离/编排（**fork**、跨进程持久后台 Agent 等）仍属独立里程碑；**工作树级**隔离、**`run_in_background` v1**（进程内注册表 + **`TaskOutput`** / **`TaskStop`**）与 Claude **`Agent` 字段**对齐见 **P5**。  
- **Skill** 插件市场 / OpenClaw 全量 parity（超出 **`SKILL.md` + `Skill` 工具** 的部分，见 [Agent skills](./skills)）。  
- **Swarm / Coordinator**、插件市场、遥测、语音、浏览器工具等。

扩大或缩小 MVP 边界时，请同步更新本节与下节验收条目。

## 最小 MVP 验收场景

每条场景后，在对应任务的 **`~/.anycode/tasks/<task_id>/output.log`** 中确认：

- **`[task_start]`**、**`[turn_start]`**（或等价 turn 标记）。  
- 若预期走工具链：**`[tool_call_start]`**（见 [run / REPL / TUI](./cli-sessions)）。  
- 非 TUI：**`== summary ==`** 段或运行时说明的未生成 summary 条件。

**场景 A**：只读检索（Glob + Grep + FileRead）。  
**场景 B**：小范围写改（Edit 或 FileWrite）。  
**场景 C**：Bash 与项目命令。  
**场景 D**：WebFetch 或 WebSearch。  
**场景 E（推荐）**：z.ai 首轮工具调用 — 配置 **`zai_tool_choice_first_turn`** 或 **`ANYCODE_ZAI_TOOL_CHOICE_FIRST_TURN=1`**。  
**场景 F**：持久记忆注入（**`memory.backend=file`**）。

通过后即认为满足本节 MVP 边界；MCP/LSP/子 Agent 等不在此列。

## 工具目录与阶段矩阵（P0–P8）

**单一事实来源**

- 注册与常量：[`crates/tools/src/catalog.rs`](../../../crates/tools/src/catalog.rs)  
- 装配：[`crates/cli/src/bootstrap/mod.rs`](../../../crates/cli/src/bootstrap/mod.rs)  
- Agent 子集：[`crates/agent/src/agents.rs`](../../../crates/agent/src/agents.rs)  

**命名对照（节选）**

| 参考（类/模块名） | 模型工具名（API） |
|------------------|------------------|
| FileEditTool | **Edit** |
| SyntheticOutputTool | **StructuredOutput** |
| BriefTool | **SendUserMessage**（别名 **Brief**） |
| MCPTool | **mcp** |
| AgentTool | **Agent**；legacy **Task** |

**阶段矩阵（摘要）**

| 阶段 | 工具（API 名） | anyCode 状态 |
|------|----------------|-------------|
| P0 | 模块拆分、`ToolServices`、`build_registry` | **完成** |
| P1 | Edit, NotebookEdit, TodoWrite 等 | **完成** |
| P2 | WebFetch, WebSearch | **完成** |
| P3 | mcp, ListMcpResourcesTool, ReadMcpResourceTool, McpAuth | **v1**：`tools-mcp` + **`ANYCODE_MCP_COMMAND`** / **`ANYCODE_MCP_SERVERS`**；deny 规则；动态 **`mcp__<slug>__authenticate`** |
| P4 | LSP | **部分**：`tools-lsp` + **`ANYCODE_LSP_COMMAND`** 时转发；未启用 stub |
| P5 | Agent, Skill, SendMessage, Task(legacy) | **Skill v1** 已落地；**Agent** / 旧 **Task** 嵌套 **`AgentRuntime`**（**`agent_type`** / **`subagent_type`** 选工具面，嵌套深度有上限）；支持 Claude 式 **`model`**、**`isolation: worktree`**（临时 git worktree）、**`cwd`** 解析为**绝对路径**；**`run_in_background: true`** 立即返回 **`status: started`**，嵌套在后台任务中执行 — 用 **`TaskOutput`** 轮询、用 **`TaskStop`** + UUID（嵌套 **turn / 工具** 边界协作式标志位 + **`AbortHandle`** 兜底；仅进程内）；**SendMessage** 写入编排快照并随 **`orchestration.json`** 持久化 |
| P6 | 编排 Task/Team/Cron 等 | **持久化 v1**：**`~/.anycode/tasks/orchestration.json`**（损坏时备份为 **`*.json.corrupt`**） |
| P7 | EnterPlanMode, Worktree, ToolSearch, Sleep, StructuredOutput | **完成** |
| P8 | PowerShell, Config, Brief, AskUserQuestion, REPL | **完成**（PowerShell 仅 Windows） |

**Cargo features**：[`crates/tools/Cargo.toml`](../../../crates/tools/Cargo.toml) 中 **`tools-mcp`** / **`tools-lsp`** 由 **`anycode`** 包透传；**`tools-http`** 预留。

## MCP 与后续里程碑（提纲）

**目标**：在 **`tools-mcp`** 启用时，将 **P3** 从占位升级为可连接的 MCP 客户端；内置工具 + MCP 动态工具合并去重；**`mcp__<server>__<tool>`** 命名与 **`security.mcp_tool_deny_patterns`** 一致过滤。

**v1 已落地（概要）**：多 stdio 会话（**`ANYCODE_MCP_SERVERS`**）、每服务器 **`mcp__<slug>__authenticate`**、配置级工具名过滤；**`ANYCODE_MCP_READ_TIMEOUT_SECS`** 可调 JSON-RPC 单行读超时，EOF/超时错误含子进程状态提示，**`McpStdioSession::stdio_child_is_running`** 健康检查。仍为 **stdio 单协议**；SSE/HTTP、完整 OAuth UI、延迟加载等为后续项。

**建议顺序**：传输层 → 协议（initialize / tools/list / tools/call）→ 在 **`bootstrap`** 单路径注册动态工具 → **`SecurityLayer`** → OAuth / **McpAuth** → 资源工具 → （进阶）延迟加载。

**代码入口**：`mcp_normalization.rs`、`mcp_tools.rs`、`mcp_stdio.rs`、`bootstrap/mcp_env.rs`；feature **`tools-mcp`**。

**P5 Skill（已落地 v1）**：多根目录 **`SKILL.md`** 扫描、**`ToolServices.skill_catalog`**、系统提示 **Available skills**、路径安全的 **`Skill`** 执行（超时、输出上限、可选最小环境）、配置 **`skills.*`**、CLI **`anycode skills list|path|init`**；可选 **`skills.expose_on_explore_plan`** 为 **explore** / **plan** 注册 **Skill**。**Agent / 旧 `Task`**：嵌套 **`AgentRuntime`**（**`SubAgentExecutor`**）。**与 Claude Code `Agent` 工具对齐（当前子集）**：入参 **`subagent_type`**（同 **`agent_type`**，**`Explore`/`Plan`/`general-purpose`** 等会规范化）、可选 **`description`**、可选 **`cwd`**（相对则相对工具调用工作目录，再 **canonicalize** 为绝对路径）、可选 **`model`**（**`sonnet`/`opus`/`haiku`** 或裸模型 id；按主会话 provider 映射）、可选 **`isolation: "worktree"`**（在系统临时目录下 **`git worktree add`**，结束后移除）；**`run_in_background: true`** 立即返回 **`status: started`**，与同步路径相同的 **`nested_task_id`** / **`output_file`** 提示，进程内注册表；用 **`TaskOutput`** 查看 **`background_status`** / 日志尾部，用 **`TaskStop`** 对 UUID：共享标志位在嵌套 **turn**、**工具** 边界以及阻塞 **`chat` / 流式 open 与 `recv`** 上与 **`tokio::select!`** 竞争（约 20ms 轮询，无 **`tokio-util`**；不保证 TCP 立即断开）。**`AbortHandle::abort`** 仍为兜底；阻塞在 syscall 的工具仍可能依赖 abort。同步成功时仍有 **`content`**。仍与上游有差距：**fork** 自身、跨进程持久后台、群聊式 **`SendMessage`** 等 — 后续里程碑。

**LSP、P5 其余项、OpenAI 官方客户端** 等与英文 [Roadmap](/guide/roadmap) 对称，细节见源码与上表。

## 最近已交付（参考）

- **嵌套任务协作式取消（v2 + v2.1）**：**`TaskStop`** 置位并传入嵌套 **`TaskContext`** / TUI **`coop_cancel`**；**`tokio::select!`** 与 **`chat` / `chat_stream` / 流式 recv** 竞争（约 20ms 轮询）。**`TaskResult::Failure`** **`cancelled`** → **`background_status: cancelled`**。可选后续：**`CancellationToken`**；**syscall** 阻塞仍为 **`AbortHandle`** 尽力。

## 建议的下一主线（维护者）

下一阶段宜 **每次只选一条** 里程碑级主线（避免两条大块重构并行）。按主线拆 **GitHub issue** 跟踪。

| 主线 | 目标（可拆成 issue 的起点） |
|------|---------------------------|
| **P5 Agent / Task** | 可选：编排与 **`~/.anycode/tasks/<id>/`** 布局对齐；**fork**；跨进程持久后台（超出当前进程内注册表）。 |
| **MCP 超出 stdio v1** | **部分已做**：环境变量 **`ANYCODE_MCP_READ_TIMEOUT_SECS`** 覆盖 JSON-RPC **单行读超时**；超时 / 意外 EOF 错误更清晰（含子进程退出）；**`McpStdioSession::stdio_child_is_running`** 健康检查。**仍待**：更完整的 stdio 生命周期 / 重连；无 GUI 下 **McpAuth** / OAuth；资源工具体验打磨。 |
| **通道 AskUserQuestion** | 微信 / Telegram / Discord 卡片或键盘选题（新 host 实现）。 |

**文档说明：**显式模型指令文件路径 **仅** 通过环境变量 **`ANYCODE_MODEL_INSTRUCTIONS_FILE`** 指定；JSON 中的 `model_instructions` **只**控制自动发现 — 见 [配置与安全](./config-security.md)。

## 相关

- [架构](./architecture)  
- [排错](./troubleshooting.md)  

English: [Roadmap](/guide/roadmap).
