---
title: 路线图
description: MVP 范围、验收场景、工具阶段矩阵与 MCP/LSP 等后续里程碑。
summary: 发布边界、如何验收、P0–P8 与后续工作入口。
read_when:
  - 规划版本或对比 anyCode 与其它终端 Agent。
  - 开发 MCP、LSP、子 Agent 相关功能。
---

# 路线图

本文合并原 **MVP 范围**、**MVP 验收**、**工具与阶段（tools-parity）**、**待实现清单（roadmap-stubs）**、**MCP 后续（mcp-postmvp）** 的要点。**代码事实来源**仍以 `crates/tools/src/catalog.rs`、`crates/cli/src/bootstrap/mod.rs`、`crates/agent/src/agents.rs` 为准。

## 最小 MVP 范围（已冻结）

**MVP 内**

- 单一工作区内 **读/写/改文件、Glob/Grep、Bash**（及文档已列的 P1/P2/P7/P8 内置工具）。  
- **审批 / 沙箱**（`SecurityLayer` 与配置项）。  
- **z.ai（OpenAI 兼容）与 Anthropic** 至少一条日常可用的 tool-calling 路径。  
- **执行期落盘日志**（`~/.anycode/tasks/<id>/output.log`）与 **结束 summary**（非 TUI 直出场景）。  
- **CLI**：`run`、默认 TUI、`repl`、可选 `daemon` HTTP。

**MVP 外**（独立里程碑，**不阻塞**上述 MVP 发布）

- **MCP** 完整产品形态（SSE/HTTP、完整 OAuth UI、延迟加载等）超出当前 stdio **v1** 范围的部分。  
- **LSP** 完整子进程故事（实验性 `tools-lsp` 之外）。  
- **子 Agent（`Agent` 工具）** 完整上下文隔离与权限继承。  
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
| P5 | Agent, Skill, SendMessage, Task(legacy) | **Skill v1**：`SKILL.md` 扫描、提示注入、**`Skill`** 硬化、**`skills.*`**、**`anycode skills`**；Agent/Task 仍为 stub / 内存 |
| P6 | 编排 Task/Team/Cron 等 | **持久化 v1**：**`~/.anycode/tasks/orchestration.json`**（损坏时备份为 **`*.json.corrupt`**） |
| P7 | EnterPlanMode, Worktree, ToolSearch, Sleep, StructuredOutput | **完成** |
| P8 | PowerShell, Config, Brief, AskUserQuestion, REPL | **完成**（PowerShell 仅 Windows） |

**Cargo features**：[`crates/tools/Cargo.toml`](../../../crates/tools/Cargo.toml) 中 **`tools-mcp`** / **`tools-lsp`** 由 **`anycode`** 包透传；**`tools-http`** 预留。

## MCP 与后续里程碑（提纲）

**目标**：在 **`tools-mcp`** 启用时，将 **P3** 从占位升级为可连接的 MCP 客户端；内置工具 + MCP 动态工具合并去重；**`mcp__<server>__<tool>`** 命名与 **`security.mcp_tool_deny_patterns`** 一致过滤。

**v1 已落地（概要）**：多 stdio 会话（**`ANYCODE_MCP_SERVERS`**）、每服务器 **`mcp__<slug>__authenticate`**、配置级工具名过滤。仍为 **stdio 单协议**；SSE/HTTP、完整 OAuth UI、延迟加载等为后续项。

**建议顺序**：传输层 → 协议（initialize / tools/list / tools/call）→ 在 **`bootstrap`** 单路径注册动态工具 → **`SecurityLayer`** → OAuth / **McpAuth** → 资源工具 → （进阶）延迟加载。

**代码入口**：`mcp_normalization.rs`、`mcp_tools.rs`、`mcp_stdio.rs`、`bootstrap/mcp_env.rs`；feature **`tools-mcp`**。

**P5 Skill（已落地 v1）**：多根目录 **`SKILL.md`** 扫描、**`ToolServices.skill_catalog`**、系统提示 **Available skills**、路径安全的 **`Skill`** 执行（超时、输出上限、可选最小环境）、配置 **`skills.*`**、CLI **`anycode skills list|path|init`**；可选 **`skills.expose_on_explore_plan`** 为 **explore** / **plan** 注册 **Skill**。**Agent / 旧 Task** 仍为 stub。

**LSP、P5 其余项、OpenAI 官方客户端** 等与英文 [Roadmap](/guide/roadmap) 对称，细节见源码与上表。

## 相关

- [架构](./architecture)  
- [排错](./troubleshooting.md)  

English: [Roadmap](/guide/roadmap).
