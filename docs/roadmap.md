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

- **审计与清理**：移除默认路径未接线模块 `daemon_http`、`virtual_scroll`；主路径低风险去重（`main.rs`、`tui/run/event.rs`、`bootstrap/mod.rs`）；`LSP` / `MCP` / `AskUserQuestion` / `REPL` 降级返回统一 `status` / `hint`。  
- **会话与用量**：流式 REPL 与全屏 TUI 对齐 **`TurnTokenUsage`** / **`TurnOutput.usage`**；HUD 与 **`/context`** 同源；**`/export`**、**`/cost`**（免责声明 + 与 context 一致的用量行）。  
- **Inline 退出 scrollback**：**`ANYCODE_STREAM_EXIT_SCROLLBACK_DUMP`** 支持 `0` / `anchor` / `full`（默认 full）；`anchor` 与 `turn_transcript_anchor` → **`ReplLineState::stream_exit_dump_anchor`**；**`/clear`** 重置 anchor。  
- **文档站**：`cli-sessions` 默认入口、TUI vs `repl`、上述斜杠命令与环境变量与实现对齐。  
- **HTTP daemon**：**不恢复** — 见 [ADR 003](adr/003-http-daemon-deprecated.md)。

---

## 3. 已完成（原 §3 三条）

| 主题 | 状态（简） |
|------|------------|
| 子 Agent 真异步 **v1** | **`run_in_background`** + **`TaskOutput`** / **`TaskStop`**（进程内注册表；**`TaskStop`** 置协作式标志 + **`AbortHandle`** 兜底）。 |
| **AskUserQuestion** | TTY dialoguer、流式 REPL、全屏 TUI；无 host 时 **`unsupported_host`**。 |
| **LSP 一等配置** | **`config.json` `lsp`** + 文档；回退 **`ANYCODE_LSP_COMMAND`**。 |

**Issue [#3](https://github.com/qingjiuzys/anycode/issues/3)** 正文草稿仍见 [`issue-drafts/001-ask-user-question.md`](issue-drafts/001-ask-user-question.md)（通道卡片选题为非目标）。

---

## 4. 下一迭代（v2 建议）

| 主题 | 完成定义（简） |
|------|----------------|
| **嵌套任务协作式取消** | **部分已落地**：共享 **`Arc<AtomicBool>`** 经嵌套 **`TaskContext`**，在 **`execute_task`** / **`execute_turn_from_messages`** 的 **turn** 与 **工具** 边界轮询；失败 error **`cancelled`** 对应 **`background_status: cancelled`**。**仍待**：单次 LLM / 流式请求内 **`select!`** 级取消；syscall 阻塞仍靠 **`AbortHandle`**。 |
| **跨进程 / 持久后台 Agent** | 与 Claude 完整 parity 的队列或等价语义（超出当前进程内 **`HashMap`**）。 |
| **通道 AskUserQuestion** | 微信 / Telegram / Discord 上卡片或键盘选题（需独立设计与鉴权）。 |

---

## 5. 后续（Later，不展开实现细节）

- **Transcript 虚拟滚动**：复启前需定义性能目标与负载模型；基线见 [`tui-smoothness-baseline.md`](tui-smoothness-baseline.md) 末尾 backlog 段。  
- **`crates/onboard`**：独立 crate 或并入 CLI — 需单独决议或 ADR。

---

## 6. 已拍板

| 决策 | 记录 |
|------|------|
| **不提供 / 不恢复 HTTP `anycode daemon`** | [ADR 003](adr/003-http-daemon-deprecated.md) |

---

## 7. 待决策

| 主题 | 备注 | ADR / 下一步 |
|------|------|----------------|
| 会话 **rewind** / 撤销展示 | 与 `tui-sessions` 快照格式兼容性 | [ADR 004](adr/004-session-rewind.md)（Proposed） |
| **`/clear` vs 纯文本 transcript 缓冲** | 是否需独立于 agent messages 的视口重置 | [ADR 005](adr/005-repl-clear-vs-transcript.md)（Proposed） |
| **virtual scroll** | 见 §4 | [ADR 006](adr/006-transcript-virtual-scroll-rfc.md)（Proposed） |

---

## 8. 相关链接

- [`architecture.md`](architecture.md) — 维护者分层与流式/TUI 会话表  
- [`docs/README.md`](README.md) — ADR 索引与文档地图  
- 仓库：<https://github.com/qingjiuzys/anycode>
