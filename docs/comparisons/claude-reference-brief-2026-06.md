# Claude Code 参考简报（2026-06）

维护者用：记录 anyCode 相对 **Claude Code TS 暴露归档** 与 **社区 claude-code-rust** 的差距与可借鉴点。  
**不**直接移植 TS 源码实现；产品 MVP 仍以 [docs-site/guide/roadmap.md](../docs-site/guide/roadmap.md) 为准；**可执行 backlog** 写在 [roadmap.md](../roadmap.md) §3.5。

## 同步基线

| 项 | 值 |
|----|-----|
| Claude TS 路径 | 同级仓库 `../claude-code`（instructkr 研究归档，npm source map 暴露快照） |
| Claude TS 本地 HEAD | `936e6c8` — 2026-03-31 |
| Claude TS 远端 HEAD | `d229a9b` — `git fetch` 因网络未完成；下次 pull 后更新本表 |
| Claude Rust 路径 | 同级仓库 `../claude-code-rust`（社区 Rust 重构，非 Anthropic 官方） |
| Claude Rust HEAD | `4b87a363` — 2026-05-20（已 fast-forward） |
| anyCode 终端对照 | [references/claude-code-rust-stream-repl.md](references/claude-code-rust-stream-repl.md) |

**性质说明**：Claude TS 仓库 README 明确为**研究/架构分析**用途，非 clean-room 重写授权来源。anyCode 只借鉴**产品模式与 UX 语义**，不复制 Ink/React 实现或专有 telemetry。

---

## 架构对照（简）

| 维度 | Claude Code TS | claude-code-rust | anyCode |
|------|----------------|------------------|---------|
| 编排权威 | QueryEngine + tool loop | 简化 API client + tools | **`AgentRuntime`**（ADR 000） |
| UI | Ink + React（~140 components） | egui GUI + colored line REPL | ratatui **Stream REPL** + dashboard |
| 工具注册 | `src/tools.ts` + permission context | `src/tools/` + MCP | `crates/tools` registry + SecurityLayer |
| 斜杠命令 | `src/commands.ts`（~50+） | REPL 内置 `.help` 等 | REPL 斜杠 + dashboard composer |
| 权限 | `permissionValidation.ts` + modes | 基础配置 | SecurityLayer + deny/allow + approval |
| 子 Agent | AgentTool + swarm/tmux | services/agents | `run_in_background` v1 进程内 |
| MCP | `services/mcp/` 全栈 | `src/mcp/` | `mcp_session.rs` + ADR 007 不重连 |
| 会话 | sessionStorage + rewind | memory/session | `~/.anycode/sessions` JSON |

---

## 差距矩阵（0.3 相关）

图例同 [openclaw-sync-brief-2026-05.md](../comparisons/openclaw-sync-brief-2026-05.md)：**Port** / **Partial** / **Skip** / **Done**。

### 1. 命令与发现

| Claude TS 要点 | anyCode | 决策 |
|----------------|---------|------|
| 统一 slash 解析（`parseSlashCommand`、MCP 后缀） | 分散在 REPL / channel | **Port** — 0.3-E 统一解析 + autocomplete 入口 |
| 动态 skill/plugin 命令注入 | skills catalog + dashboard | **Partial** — CLI autocomplete Later |
| `/doctor` `/status` `/context` `/cost` | doctor 子命令 + HUD 部分 | **Partial** — 扩面到 MCP/cron/channel |

### 2. 权限与工具治理

| Claude TS 要点 | anyCode | 决策 |
|----------------|---------|------|
| 权限规则语法校验 + suggestion（Bash prefix、MCP `mcp__`） | `permission_rule_parser.rs` | **Port** — 0.3-B 配置写入时校验与提示 |
| Permission modes（auto / default / plan） | approval + policy | **Partial** — 不复制 auto bypass；文档对齐 |
| ToolSearch / deferred tools | tool surface policy | **Later** |
| `tool-calls` 审计链 | evidence.jsonl + gate log | **Port** — 0.3-B `tool-calls.jsonl` |

### 3. 终端 / 会话 UX

| Claude TS / Rust 要点 | anyCode | 决策 |
|------------------------|---------|------|
| Ink 工具卡片 + executing spinner | `ReplLineState` + dock_render | **Partial** — 0.3-E HUD 收敛 |
| 平滑滚动 / scrollbar（claude-code-rust `chat.rs`） | stream_viewport | **Partial** — ADR 006 前只做负载模型 |
| `/rewind` 会话撤销 | ADR 004 Proposed | **Later** — 先统一语义 |
| session restore / resume chooser | sessions JSON load | **Partial** — dashboard 已有 replay |
| 启动预取（keychain、MCP、policy） | bootstrap 部分 | **Later** — 非 0.3 阻塞 |

### 4. 多 Agent / 团队

| Claude TS 要点 | anyCode | 决策 |
|----------------|---------|------|
| swarm / tmux / team backends | 无内嵌 OMX | **Skip** — 外接 oh-my-codex |
| TaskCreate/Update/List 工具面 | goal_engine / orchestration | **Partial** |
| Coordinator mode | 单一 AgentRuntime | **Skip** |

### 5. 明确不做

- Ink/React 终端栈、Bun 运行时、GrowthBook/telemetry 全栈
- Claude 专有 OAuth / MDM / remote managed settings
- claude-code-rust 的 egui 桌面壳（anyCode 已有 dashboard + Tauri 线）

---

## 0.3 映射（→ roadmap §3.5）

| Claude 借鉴点 | anyCode 0.3 包 |
|---------------|----------------|
| QA 式 approval-denial / no-fake-progress 场景 | 0.3-A Eval |
| permissionValidation + tool audit | 0.3-B Tool Governance |
| MCP doctor/status 信息架构 | 0.3-C MCP Doctor |
| slash autocomplete + HUD 状态机 | 0.3-E Terminal UX |

---

## 相关文档

- [roadmap.md](../roadmap.md) §3.5 — 0.3 执行范围
- [openclaw-sync-brief-2026-05.md](../comparisons/openclaw-sync-brief-2026-05.md) — OpenClaw 对标
- [references/claude-code-rust-stream-repl.md](references/claude-code-rust-stream-repl.md) — Stream REPL 模块对照
