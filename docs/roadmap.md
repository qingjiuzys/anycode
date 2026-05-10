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

- **Setup / 配置**：交互式记忆向导（`file` / `hybrid` / `pipeline` / HTTP 向量 / 可选 **`embedding-local`**）、**`noop` 禁用记忆** 向导项；实现见 `setup_memory.rs` / `app_config`。  
- **Cron / 调度器**：`scheduler.lock`；WeChat / Telegram / Discord 内嵌 `run_builtin_scheduler`；`channel_task` + Cron 工具链。  
- **会话与 CLI**：协作取消、流式 REPL 模块化、**Telegram `AskUserQuestion`**（`tg_ask` 内联键盘）、MCP stdio 超时 + **`mcp_stdio_dead` 快路径**、会话通知、HUD/`/context`/`/export`/`/cost`、审计清理等（细节见 `CHANGELOG` 与文档站）。

---

## 3. 已完成（摘要表）

| 主题 | 状态（简） |
|------|------------|
| 子 Agent 真异步 **v1** | **`run_in_background`** + **`TaskOutput`** / **`TaskStop`**（进程内注册表；**`TaskStop`** 置协作式标志 + **`AbortHandle`** 兜底）。 |
| **嵌套协作取消 v2+v2.1** | 见 §2；**`cancelled`** → **`background_status: cancelled`**；HTTP / syscall 边界见 **`CHANGELOG`**。 |
| **AskUserQuestion** | TTY dialoguer、流式 REPL、全屏 TUI；**Telegram 通道**内联键盘（`tg_ask`）；无 host 时 **`unsupported_host`**。 |
| **LSP 一等配置** | **`config.json` `lsp`** + 文档；回退 **`ANYCODE_LSP_COMMAND`**。 |

**Issue [#3](https://github.com/qingjiuzys/anycode/issues/3)** 正文草稿仍见 [`issue-drafts/001-ask-user-question.md`](issue-drafts/001-ask-user-question.md)（通道卡片选题为非目标）。

---

## 4. 下一迭代候选

| 主题 | 完成定义（简） |
|------|----------------|
| **MCP stdio 超出 v1（续）** | **本版**：`ANYCODE_MCP_CALL_TIMEOUT_SECS`、子进程已退出时 **快速失败**（`mcp_stdio_dead`）；**[ADR 007](adr/007-mcp-session-reconnect-policy.md)**（**Accepted**，仅政策）— **不静默重连**；可选配置位与未来受控重连见 ADR。 |
| **跨进程 / 持久后台 Agent** | 与 Claude 完整 parity 的队列或等价语义（超出当前进程内 **`HashMap`**）。仅排期，见 §5。 |
| **通道 AskUserQuestion** | **Telegram（已交付）**：内联键盘 + callback，[`tg_ask`](crates/cli/src/tg_ask.rs)；**续**：Discord/微信等见 [ADR 008](adr/008-channel-ask-user-question-phasing.md)。 |

---

## 5. 后续（Later，不展开实现细节）

- **跨进程 / 持久后台 Agent**：独立 spike / ADR 后再写实现；与 §4 表同步。  
- **Transcript 虚拟滚动（ADR 006）**：复启前需性能目标与负载模型；基线见 [`term-smoothness-baseline.md`](term-smoothness-baseline.md) 末尾。  
- **会话 rewind（ADR 004）/ `/clear` 语义（ADR 005）**：暂缓至产品缺口明确。  
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
| **MCP stdio 受控重连（实现）** | 政策已 **Accepted**（ADR 007）；**代码层自动重连**仍待 flag + 原子工具表更新后再开 | [ADR 007](adr/007-mcp-session-reconnect-policy.md) |
| **通道 AskUserQuestion 扩展** | Telegram 已 MVP；Discord / 微信文本回落等 | [ADR 008](adr/008-channel-ask-user-question-phasing.md) |
| 会话 **rewind** / 撤销展示 | 与 `sessions` 快照格式兼容性 | [ADR 004](adr/004-session-rewind.md)（Proposed）— **暂缓**：无实现排期前保持 Proposed，改快照前必读。 |
| **`/clear` vs 纯文本 transcript 缓冲** | 是否需独立于 agent messages 的视口重置 | [ADR 005](adr/005-repl-clear-vs-transcript.md)（Proposed）— **暂缓**：流式 REPL 已有 `turn_transcript_anchor` / `stream_exit_dump_anchor`，产品缺口再开。 |
| **virtual scroll** | 见 §5 Later | [ADR 006](adr/006-transcript-virtual-scroll-rfc.md)（Proposed）— **暂缓**：与 [`term-smoothness-baseline.md`](term-smoothness-baseline.md) 负载模型挂钩后再审。 |

---

## 8. 相关链接

- [`architecture.md`](architecture.md) — 维护者分层与流式/TUI 会话表  
- [`docs/README.md`](README.md) — ADR 索引与文档地图  
- 仓库：<https://github.com/qingjiuzys/anycode>
