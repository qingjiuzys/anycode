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
- **Cron / 微信**：`scheduler.lock`；桥内嵌调度器；`CronCreate` 本地墙钟→UTC；**先推送微信再跑 agent**（`cron_notify`）；`weekday *` 避免一次性任务永不触发。  
- **微信 UX**：不再向会话推送 `🔧`/`✓` 工具进度行。  
- **会话与 CLI**：协作取消、流式 REPL 模块化、**Telegram `AskUserQuestion`**（`tg_ask`）、MCP stdio 超时 + **`mcp_stdio_dead`**、会话通知、HUD/`/context`/`/export`/`/cost` 等。  
- **OpenClaw 对标**：本地已拉至 **2026.5.19** 线（`ddeaebfc`）；见 [`openclaw-sync-brief-2026-05.md`](openclaw-sync-brief-2026-05.md)、[`weixin-plugin-parity.md`](weixin-plugin-parity.md)。  
- **5.19 小步**：stream→chat 不重复 assistant；DeepSeek `anyOf` schema 规范化；pipeline 向量/嵌入降级 WARN；`cron-runs.jsonl` + `CronCreate` 校验；流式 REPL resize tick 顺序；`WebFetch` 私网主机 + 重定向跳数上限；provider kebab 别名补全（`zhipu-ai` 等）；微信出站 `send_text` 重试。

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

## 4. 生产级下一阶段（2026-05 起）

**方向切换**：OpenClaw 5.19 parity 的短线修补已基本完成；下一阶段按生产级能力推进，避免继续以 provider alias / 小单测数量作为主目标。每个 Epic 的完成定义必须包含：用户场景、失败场景、targeted tests、日志/诊断入口、文档/CHANGELOG、禁用或回滚边界。

| # | Epic | 主题 | 完成定义（简） |
|---|------|------|----------------|
| A | Eval / Release | **可重复评测与发布验收** | `anycode eval` 最小场景、mock LLM / fixture repo、release readiness 文档；CI 分层 |
| B | Security / Tools | **工具治理控制面** | `tool-calls.jsonl` 审计、WebFetch/MCP sanitizer 与 scanner、Bash env policy |
| C | Agent Runtime | **长任务与持久后台诊断** | overflow single-retry、compaction checkpoint metadata、background task state |
| D | MCP / LSP | **受控 MCP 与工具生态** | `doctor mcp` / `mcp status`、ADR 007 controlled reconnect、resource UX |
| E | Automation / Cron | **可审计自动化** | stable cron session、`cron runs` 查询、failure destination、per-job tool profile |
| F | Channels | **IM 生产可靠性** | WeChat parity closure、Discord / WeChat AskUserQuestion、outbound queue、`channel status` |
| G | Memory / Terminal / Ops | **上下文、终端与诊断** | evidence index、memory doctor、transcript 负载模型、error taxonomy、doctor 命令 |

**2026-05 已交付**：OpenClaw 对标简报、流式 REPL resize 不变量、DeepSeek `anyOf` schema 规范化、stream→chat fallback transcript、pipeline 向量 WARN、[`cron-runs.jsonl`](cron-observability.md)、`CronCreate` 校验 + IANA 时区、WebFetch 私网/DNS/redirect 防护、provider kebab 别名、微信出站重试、cli_smoke 隔离。详见 [`openclaw-sync-brief-2026-05.md`](openclaw-sync-brief-2026-05.md)。

### 4.1 执行顺序

1. **Epic A** 先行：评测 harness 会约束后续大改，避免只靠 `cargo test --workspace`。
2. **Epic B + D** 第二批：工具/MCP 是生产风险面，先做审计和诊断，再做 reconnect。
3. **Epic C** 第三批：overflow retry 与 durable state，不承诺一步到位恢复执行。
4. **Epic E + F** 第四批：cron stable session、WeChat / Discord 可靠性。
5. **Epic G** 收束 release candidate：memory evidence、terminal 负载模型、doctor / release readiness。

---

## 5. 后续（Later）

- **真正恢复执行的跨进程后台 Agent**：先完成 diagnostic state，再决定是否恢复执行。
- **Telegram 可选 draft 工具进度**（默认关）。
- **memory-wiki / dreaming 全栈**：仍不做；只保留 graph memory spike。
- **Transcript 虚拟滚动（ADR 006）**：先跑负载模型，再实现。
- **会话 rewind（ADR 004）/ `/clear`（ADR 005）**：先统一语义，再改快照。
- **Webhook / TaskFlow / SQLite ledger**：除非 cron 使用场景明确，不复制 Gateway。
- **`crates/onboard`** — 单独决议。

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
