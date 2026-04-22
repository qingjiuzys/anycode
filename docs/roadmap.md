# anyCode 维护者路线图（SSOT）

本文档是 **执行层 backlog 的单一事实来源**：最近交付、下一迭代、后续池、已拍板与待决策。  
产品级 **MVP 边界、工具 P0–P8 矩阵、验收场景** 仍以文档站为准（避免在本文件重复整张矩阵）：

| 语言 | 源码路径 |
|------|-----------|
| English | [`docs-site/guide/roadmap.md`](../docs-site/guide/roadmap.md) |
| 中文 | [`docs-site/zh/guide/roadmap.md`](../docs-site/zh/guide/roadmap.md) |

协作约定：**迭代任务与决策状态只改本文件（及 `docs/adr/`）**；不要在 `docs-site` 再维护一份相同的 now/next/later 列表。

在线浏览本文件：<https://github.com/qingjiuzys/anycode/blob/main/docs/roadmap.md>

---

## 1. 文档治理（落地规则）

1. **分工**  
   - **`docs-site/.../roadmap.md`**：产品叙事、MVP、工具阶段矩阵、MCP/LSP 提纲。  
   - **`docs/roadmap.md`（本文件）**：可执行的 now/next/later、已完成摘要、决策表。

2. **Next**  
   - 建议保持 **≤7** 条；溢出移到 **Later** 或拆成独立 GitHub issue。

3. **Later**  
   - 每 **1～2 个月**扫一次：长期无进展则写入 ADR（明确不做或合并主题），避免清单无限膨胀。

4. **待决策**  
   - 本文件只保留 **表格级摘要**；选项、取舍、后果写在 **`docs/adr/`**。

5. **最近已交付**  
   - 保留约 **两个版本窗口** 的摘要即可；更老的历史可查 `CHANGELOG.md` 或 git。

---

## 2. 最近已交付（摘要）

- **会话外向通知**：`config.json` **`notifications`** — 工具结果后 / 无后续 tool_calls 的 assistant 回合结束时，可选 **HTTP POST JSON** 或 **`shell_command`（stdin 为 JSON）**；与 **`memory.pipeline.hook_*`**（归根 ingest）独立；头 **`${ENV_VAR}`** 展开；失败不阻断 **`AgentRuntime`**（见 [`architecture.md`](architecture.md)）。  
- **流式 REPL 模块化**：Inline dock / viewport / 事件 / 任务循环等拆至 `crates/cli/src/repl/`、`tasks/stream_repl_loop.rs`；布局与术语见 [`stream-repl-layout.md`](stream-repl-layout.md)；与 claude-code-rust 对照见 [`references/claude-code-rust-stream-repl.md`](references/claude-code-rust-stream-repl.md)。  
- **微信桥中断提示**：**`SessionState::Processing`** 下新非斜杠消息 **abort** 上一段任务后发送 **`wx-interrupt-new-msg`**（Fluent）。  
- **审计与清理**：移除默认路径未接线模块 `daemon_http`、`virtual_scroll`；主路径低风险去重（`main.rs`、`tui/run/event.rs`、`bootstrap/mod.rs`）；`LSP` / `MCP` / `AskUserQuestion` / `REPL` 降级返回统一 `status` / `hint`。  
- **会话与用量**：流式 REPL 与全屏 TUI 对齐 **`TurnTokenUsage`** / **`TurnOutput.usage`**；HUD 与 **`/context`** 同源；**`/export`**、**`/cost`**（免责声明 + 与 context 一致的用量行）。  
- **Inline 退出 scrollback**：**`ANYCODE_TERM_EXIT_SCROLLBACK_DUMP`** 支持 `0` / `anchor` / `full`（默认 full）；`anchor` 与 `turn_transcript_anchor` → **`ReplLineState::stream_exit_dump_anchor`**；**`/clear`** 重置 anchor。  
- **文档站**：`cli-sessions` 默认入口、TUI vs `repl`、上述斜杠命令与环境变量与实现对齐。  
- **HTTP daemon**：**不恢复** — 见 [ADR 003](adr/003-http-daemon-deprecated.md)。  
- **嵌套协作取消 v2+v2.1**：**`TaskStop`** + **`Arc<AtomicBool>`**；**`execute_task` / `execute_turn_from_messages`** 在 turn、工具、**`chat`、流式 open+recv** 上与取消竞争（~20ms 轮询）。  
- **主会话协作取消（TUI / stream REPL / stdio 行模式）**：全屏 TUI、TTY **`repl` Inline**、非 TTY stdio 逐行入口在回合进行中可将 **Ctrl+C** 置位 **`turn_coop_cancel`**，与上条同一 **`execute_turn_from_messages`** 机制；TUI 空闲仍为连按 **Ctrl+C** 退出。  
- **MCP stdio（部分加固）**：**`ANYCODE_MCP_READ_TIMEOUT_SECS`**（按行读）；可选 **`ANYCODE_MCP_CALL_TIMEOUT_SECS`**（整次 **`tools/call`**）；超时 / EOF 错误含 **server** / **子进程退出**；**`McpStdioSession::stdio_child_is_running`**。  
- **流式 REPL 总结可见性**：**`execute_turn_from_messages`** 在 **`llm_summary_receipt`** 路径将总结 **追加为末条 Assistant** 写入共享 **`messages`**，主区由 **`build_stream_turn_plain`** 渲染；并与 **`execute_task`** 对齐 **`[final_output]` / `== summary ==`** 任务日志及 summary 路径 **`pipeline_memory_hook_agent_turn`** / **`maybe_session_notify_agent_turn`**。**手测**：TTY **`anycode repl`** 用「多轮工具 + 末轮空正文」或易触发 summary 的模型，确认主区出现总结；Google / OpenAI 兼容各冒烟一条。

---

## 3. 已完成（摘要表）

| 主题 | 状态（简） |
|------|------------|
| 子 Agent 真异步 **v1** | **`run_in_background`** + **`TaskOutput`** / **`TaskStop`**（进程内注册表；**`TaskStop`** 置协作式标志 + **`AbortHandle`** 兜底）。 |
| **嵌套协作取消 v2+v2.1** | 见 §2；**`cancelled`** → **`background_status: cancelled`**；HTTP / syscall 边界见 **`CHANGELOG`**。 |
| **AskUserQuestion** | TTY dialoguer、流式 REPL、全屏 TUI；无 host 时 **`unsupported_host`**。 |
| **LSP 一等配置** | **`config.json` `lsp`** + 文档；回退 **`ANYCODE_LSP_COMMAND`**。 |

**Issue [#3](https://github.com/qingjiuzys/anycode/issues/3)** 正文草稿仍见 [`issue-drafts/001-ask-user-question.md`](issue-drafts/001-ask-user-question.md)（通道卡片选题为非目标）。

---

## 4. 下一迭代候选

| 主题 | 完成定义（简） |
|------|----------------|
| **MCP 超出 stdio v1（续）** | **仍可继续**：会话级健康 / 重连。**本版已做**：**`ANYCODE_MCP_CALL_TIMEOUT_SECS`**、**McpAuth** 无 GUI 文档（见文档站 **config-security** / **troubleshooting**）。**策略**：当前 **不自动重连**（见 [`mcp-stdio-lifecycle.md`](mcp-stdio-lifecycle.md) 与文档站 troubleshooting）；受控重连需另开 ADR + 实现。 |
| **跨进程 / 持久后台 Agent** | 与 Claude 完整 parity 的队列或等价语义（超出当前进程内 **`HashMap`**）。 |
| **通道 AskUserQuestion** | 微信 / Telegram / Discord 上卡片或键盘选题（需独立设计与鉴权）。**深做候选**：优先 **Telegram** 或 **Discord**（内联键盘 / 组件 API 成熟）；微信仍以文本与现有 **`PermissionBroker`** 为主；设计草案见 [`channel-ask-user-question-spike.md`](channel-ask-user-question-spike.md)。**实现切片**：`pub(crate)` 宿主或枚举扩展 **`AskUserQuestionHost`**；不接第二通道前不升 public trait。 |

---

## 5. 后续（Later，不展开实现细节）

- **Transcript 虚拟滚动**：复启前需定义性能目标与负载模型；基线见 [`term-smoothness-baseline.md`](term-smoothness-baseline.md) 末尾 backlog 段。  
- **`crates/onboard`**：独立 crate 或并入 CLI — 需单独决议或 ADR。

---

## 6. 已拍板

| 决策 | 记录 |
|------|------|
| **不提供 / 不恢复 HTTP `anycode daemon`** | [ADR 003](adr/003-http-daemon-deprecated.md) |
| **MCP stdio 长驻会话不自动重连** | 子进程退出 / EOF / 超时后由用户修正命令或重启 CLI；见 [`mcp-stdio-lifecycle.md`](mcp-stdio-lifecycle.md) |

---

## 7. 待决策

| 主题 | 备注 | ADR / 下一步 |
|------|------|----------------|
| 会话 **rewind** / 撤销展示 | 与 `sessions` 快照格式兼容性 | [ADR 004](adr/004-session-rewind.md)（Proposed）— **暂缓**：无实现排期前保持 Proposed，改快照前必读。 |
| **`/clear` vs 纯文本 transcript 缓冲** | 是否需独立于 agent messages 的视口重置 | [ADR 005](adr/005-repl-clear-vs-transcript.md)（Proposed）— **暂缓**：流式 REPL 已有 `turn_transcript_anchor` / `stream_exit_dump_anchor`，产品缺口再开。 |
| **virtual scroll** | 见 §5 Later | [ADR 006](adr/006-transcript-virtual-scroll-rfc.md)（Proposed）— **暂缓**：与 [`term-smoothness-baseline.md`](term-smoothness-baseline.md) 负载模型挂钩后再审。 |

---

## 8. 相关链接

- [`architecture.md`](architecture.md) — 维护者分层与流式/TUI 会话表  
- [`docs/README.md`](README.md) — ADR 索引与文档地图  
- 仓库：<https://github.com/qingjiuzys/anycode>
