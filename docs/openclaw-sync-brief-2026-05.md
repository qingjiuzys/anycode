# OpenClaw 对标简报（2026-05）

维护者用：记录 anyCode 相对上游 OpenClaw 的差距与取舍。产品 MVP 边界仍以 [docs-site/guide/roadmap.md](../docs-site/guide/roadmap.md) 为准；**可执行 backlog** 写在 [roadmap.md](roadmap.md)。

## 同步基线

| 项 | 值 |
|----|-----|
| OpenClaw 路径 | 同级仓库 `../openclaw`（`llm-cli/openclaw`） |
| 旧基线 | `5aa8579` — 2026-04-08（约 2026.4.8 线） |
| 当前 HEAD | `ddeaebfc` — 2026-05-18 |
| 较 `v2026.4.8` | 约 +21,926 commits |
| CHANGELOG 锚点 | [2026.5.19](https://github.com/openclaw/openclaw/blob/main/CHANGELOG.md#2026519) |
| anyCode 对照提交 | `8f7f31a`（cron 微信投递）、`7c77335`（去掉微信工具进度行） |

**节奏**：每 2–4 周 `git pull` + 在本文件末尾追加「增量」小节（Unreleased + 最新 5.x 块即可）。

---

## 七领域差距矩阵

图例：**Port** = 建议在 anyCode 借鉴实现；**Partial** = 语义/子集；**Skip** = 架构边界不做；**Done** = 已有等价能力。

### 1. Providers / 模型 / 推理

| OpenClaw（5.x 要点） | anyCode | 决策 |
|----------------------|---------|------|
| Codex app-server 主路径、动态工具桥 | 自有 `AgentRuntime` + provider 抽象 | **Skip** Gateway/Codex 托管 |
| Z.AI GLM manifest、DeepSeek `anyOf` 规范化（5.19） | `normalize_tool_parameters_schema` in z.ai OpenAI path | **Done**（2026-05） |
| Failover 时 transcript 不重复（5.19） | stream→chat pop placeholder | **Done**（2026-05） |
| `openclaw infer` CLI hub | `anycode model` | **Partial** — 文档对齐即可 |
| Prompt cache / thinking 元数据 | 部分 provider 支持 | **Later** |

### 2. Agent 运行时 / Subagent / 回复

| OpenClaw（5.x 要点） | anyCode | 决策 |
|----------------------|---------|------|
| Subagent announce 恢复、stale completion（5.19） | `run_in_background` v1 进程内 | **Partial** — 不做 Gateway 注册表 |
| 默认 steer 中途注入用户消息（4.29） | REPL/通道可取消 | **Later** — 通道 steer 需 ADR |
| Context overflow 合并恢复（4.7） | compaction 有 | **Port** — 评估单 pass 恢复 |
| 剥离 tool XML / function_response 泄漏（5.14） | 微信 sanitize | **Port** — 统一到 transcript |
| Fallback 不重复 assistant 条（5.19） | stream→chat 先 pop 占位再 push | **Done**（2026-05） |

### 3. Memory / Compaction / Dreaming

| OpenClaw（5.x 要点） | anyCode | 决策 |
|----------------------|---------|------|
| memory-wiki、人物图谱、Active Memory | file/hybrid/pipeline | **Skip** 全栈 |
| Dreaming / REM / UI | 无 | **Skip** |
| sqlite-vec 分批扫描、主线程让出（5.19） | pipeline 向量 | **Partial** — 借鉴思路若用 sqlite-vec |
| 向量降级显式 WARN（4.7） | pipeline `tracing::warn` | **Done**（2026-05） |

### 4. Channels（IM）

| OpenClaw（5.x 要点） | anyCode | 决策 |
|----------------------|---------|------|
| `@tencent-weixin/openclaw-weixin@2.4.3` | 原生 Rust `wx/bridge.rs` | **Port** — [weixin-plugin-parity.md](weixin-plugin-parity.md) |
| Telegram draft 工具进度（5.19） | 无进度推送（微信已关） | **Skip** 默认；TG 可选 Later |
| 论坛 topic / 回复引用 / Mini App | `tg.rs`、`tg_ask` | **Partial** — 按 CHANGELOG 逐项核对 |
| WhatsApp/Slack/Discord 大量修复 | TG/Discord/微信为主 | **按需** |

**取舍说明**：anyCode 微信侧选择**不推送** `🔧/✓` 工具行（用户只要最终回复）；OpenClaw Telegram 用 **draft 预览** 且不写入 transcript——策略不同，不必强行一致。

### 5. Automation（Cron / Task）

| OpenClaw（5.x 要点） | anyCode | 决策 |
|----------------------|---------|------|
| Gateway cron：isolated、announce、failureDestination、doctor | 内嵌 `scheduler.rs` + `orchestration.json` | **Partial** |
| Cron link 到稳定 session（5.19 #83606） | 每 fire 新 task id | **Later** |
| `--tz` / IANA、per-job `--tools` | `CronCreate` local→UTC；`cron-runs.jsonl` | **Partial** — 校验 + run 日志 **Done**；IANA **Later** |
| Webhook / TaskFlow / SQLite ledger | 无 | **Skip** |

### 6. Terminal / TUI

| OpenClaw（5.x 要点） | anyCode | 决策 |
|----------------------|---------|------|
| TUI 工具卡片、commentary 隐藏、Kitty 恢复 | stream REPL ratatui | **Partial** — 借鉴行为 |
| resize 期间不重复刷行 | tick 在 draw 之后 | **Done**（2026-05） |

### 7. Security / Exec / Fetch

| OpenClaw（5.x 要点） | anyCode | 决策 |
|----------------------|---------|------|
| SSRF、重定向丢 body、fetch guard | `WebFetch` blocks private/loopback + redirect cap + strip credentials | **Partial** — host literal + redirect hop guard **Done**（2026-05）；DNS rebinding **Later** |
| Host exec env 净化 | `SecurityLayer` + Bash | **Later** |
| Gateway 禁止模型改 safeBins | 配置写盘路径不同 | **Skip** |

---

## anyCode 已覆盖（无需重复）

- 微信 CronCreate + 内嵌调度器 + `cron_notify` 先推送
- `scheduler.lock` 单实例
- Telegram `AskUserQuestion`（`tg_ask`）
- MCP stdio v1、`mcp_stdio_dead`、ADR 007 不重连政策
- 流式 REPL 模块化（`stream_repl_loop` / `stream_ratatui`）

---

## 相关文档

- [weixin-plugin-parity.md](weixin-plugin-parity.md) — npm 插件 2.4.3 与 Rust 桥对照
- [wx-streaming-bridge.md](wx-streaming-bridge.md) — 微信桥边界
- [stream-repl-layout.md](stream-repl-layout.md) — 流式 REPL 不变量
- [cron-observability.md](cron-observability.md) — `cron-runs.jsonl` 字段说明
- [roadmap.md](roadmap.md) — 执行层 now/next/later

---

## 增量（后续 pull 后追加）

### 2026-05-20（anyCode 会话）

- **Cron**：`CronCreate` 支持 IANA `schedule_timezone`（`chrono-tz` 墙钟→UTC）；文档与 CHANGELOG 同步。
- **WebFetch**：十六进制 IPv4 主机名（`0x7f000001`）与十进制字面量同样拦截。
- **Providers**：`claude-cli` / `anthropic-cli`→`anthropic`，`azure-openai`→`openai`，`venice-ai`→`venice`，`stepfun-ai` / `chutes-ai` / `sglang-ai`，`opencode-ai` / `synthetic-ai`，`litellm-ai` / `kilocode-ai`。
- **WebFetch**：IPv4-mapped loopback（`::ffff:127.0.0.1`）拦截。
- **微信桥**：入站媒体 `VIDEO` 优先于 `FILE` 单测；安全层跳过非法 deny 正则。

### 2026-05-19（anyCode 会话，续）

- **CI**：`normalize_openclaw_aliases` 失败因 `zhipu-ai` → `zhipu_ai` 未映射；已修复并扩展 kebab 别名（`deepseek-ai`、`x-ai`、`byte-plus` 等）。
- **微信桥**：出站 `send_text` 瞬态 HTTP 重试（ capped backoff）；bridge 记录 chunk 发送失败。
- **cli_smoke**：line REPL 测试使用 temp `memory.backend=noop`，避免与运行中 bridge 争用 `~/.anycode/memory.sled`（本地 WouldBlock，非 CI 回归）。
- **Agent**：compact policy 边界测试（87999 vs 88000、零窗口）。
- **Cron**：校验拒绝 7 字段表达式。

### 2026-05-19（anyCode 会话）

- **WebFetch**：DNS rebinding 防护（解析后拒绝私网/链路本地 A/AAAA）；十进制 IPv4 主机名拦截；与既有字面量 SSRF + 重定向跳校验叠加。
- **Providers**：`nim`→`nvidia`、`ernie`/`baidu`→`qianfan`、`chatgpt`/`open-ai`→`openai`、`zhipu`→`z.ai` 等别名。
- **CronCreate**：无效表达式错误含字段数与 normalized 提示。
- **微信桥**：`ref_msg` 仅 `title`、无 `message_item` 时仍输出 `[引用: …]` 行。

_（下次 OpenClaw 上游 pull 后在此继续追加。）_
