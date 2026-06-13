# anyCode 产品收敛审计（2026-06）

**目的：** 收敛为「终端 + 本地 Workbench + 个人微信」最新产品边界；列出无效/未实现、不需要、编译告警、死引用四类清单，并给出动作与风险。

**SSOT：** [`docs/architecture.md`](../architecture.md)、[`docs/roadmap.md`](../roadmap.md)、ADR、[`docs/comparisons/workbuddy-comparison-2026-06.md`](../comparisons/workbuddy-comparison-2026-06.md)。

---

## 1. 最新产品边界（Keep）

| 面 | 入口 | 模块 |
|----|------|------|
| 默认交互 | `anycode`（无子命令） | `tasks_repl` — TTY stream REPL；非 TTY 行式 stdio |
| 单次任务 | `anycode run` / `--goal` / `--workflow` | `AgentRuntime::execute_task` |
| 自动化 | `anycode scheduler`、`CronCreate` 工具 | `orchestration.json`、`cron-runs.jsonl` |
| 配置/诊断 | `setup`、`config`、`model`、`doctor`、`eval` | `app_config/`、`commands/` |
| IM 通道 | `anycode channel {wechat,telegram,discord,status}` | `channels/` |
| Workbench | `anycode dashboard` | `crates/dashboard` + `dashboard-ui` |
| Desktop | `apps/anycode-desktop`（macOS `.dmg`） | sidecar spawn dashboard，不直接调 runtime |
| 编排权威 | 唯一多轮路径 | `AgentRuntime::execute_task` / `execute_turn_from_messages`（ADR 000） |

**已移除（勿恢复）：** `daemon`、`repl`、`tui`、`list-agents`、`list-tools`、`onboard` 子命令 — ADR 003 + `cli_args` 测试。

---

## 2. 无效 / 未实现功能清单

| ID | 项 | 证据 | 建议动作 | 风险 |
|----|-----|------|----------|------|
| U1 | HTTP `anycode daemon` | ADR 003；docs-site `cli-daemon.md` | **document** 仅保留迁移说明 | 低 |
| U2 | Rerank model probe | `dashboard/src/llm_probe.rs` — `rerank probe not implemented` | **defer** 或实现 probe | 低 |
| U3 | LSP 关闭时 stub 响应 | `tools/src/lsp_tool.rs` | **keep** feature-gated | — |
| U4 | MCP 关闭时 unsupported | `tools/src/mcp_tools.rs` | **keep** feature-gated | — |
| U5 | RemoteTrigger v1 无出站 | `tools/src/orchestration.rs` | **document** partial | 中 |
| U6 | Team* 持久化无 swarm | `orchestration.rs` | **document** partial | 中 |
| U7 | Discord/WeChat AskUserQuestion | ADR 008 Proposed | **defer** | 中 |
| U8 | Cron UI 编辑/删除 | `docs/roadmap.md §7` | **defer** 产品 backlog | 低 |
| U9 | SSO/RBAC / Connector OAuth 写回 | Tier 2/3 文档 | **hide** UI 占位文案，文档标 out-of-scope | 低 |
| U10 | Production Harness M1–M8 部分 | `production-harness-hardening.md` Planned | **defer** | — |
| U11 | Session rewind / virtual scroll | ADR 004/006 Proposed | **defer** | — |
| U12 | `crates/onboard` 独立 crate | CLI `onboard` 已移除 | **remove** 或合并进 setup（单独决议） | 中 |
| U13 | Provider `placeholder_only` 目录项 | `llm/src/provider_catalog.rs` | **keep** 展示用；wizard 已跳过 | 低 |
| U14 | Slack connector 写入 | i18n 标注未实现 | **document** | 低 |

---

## 3. 不需要功能候选清单

| 动作 | 项 | 依据 |
|------|-----|------|
| **remove** | 小程序云 relay、Gateway HTTP 复制、memory-wiki/dreaming 全栈 | `workbuddy-comparison` Skip；`openclaw-sync-brief` Skip |
| **remove** | 飞书/钉钉/企微/QQ 通道 | `workbuddy-comparison` 明确不做 |
| **remove** | 腾讯 Credits / 混元绑定 | Skip |
| **hide** | Tier 2 SSO/RBAC、Connector 写回、Browser visual gates | 文档 Tier 2+；UI 不承诺 |
| **document** | Telegram/Discord 通道 | 已 shipped；WeChat 为个人微信主通道 |
| **keep experimental** | `RemoteTrigger`、`Team*`、`PowerShell`、`StructuredOutput`、`Brief` | 工具 catalog 已注册；cron/agent 可用 |
| **keep** | MCP/LSP/knowledge-embeddings | feature-gated + CI 矩阵 |
| **keep** | Desktop Tauri v0.1 | macOS sidecar；非完整离线壳 |

---

## 4. 编译告警清单（2026-06 采样）

### 4.1 Phase 1 已处理（低风险）

- 未使用 import：`dashboard` control/media/skill_market/workspace_scan；`cli` bootstrap/builtin/scheduler/text_file_prompt；`llm` tts_local 顶层 import
- 命名：`SkillCatalogish` → `skill_catalogish`
- 可见性：`cli_error::TaxonomyRow` → `pub(crate)`
- Feature-gated allow：`whisper_model_fetch`、`browser_mcp`

### 4.2 仍待 Phase 3（中高风险，勿盲删）

| 区域 | 约计 | 说明 |
|------|------|------|
| `cli/src/term/*` | ~25 | stream REPL 遗留 helper；statusline skill 可能再接 |
| `cli/src/app_config/schema/types.rs` | 若干字段 | 部分仅测试消费；auto-compact 待接线 |
| `agent/src/runtime/*` | 4 | goal_engine、logging gate 等 |
| `cli/channels/wx/*` | 若干 | CDN/deliverable/voice STT 未完成路径 |
| `tools/knowledge_tools.rs` | 1 | `services` 字段 registry 占位 |

---

## 5. 无效引用 / 死代码清单

### 5.1 Dashboard UI（已删除）

| 文件 | 原因 | 替代 |
|------|------|------|
| `SearchBox.tsx` | 无 import | `TopbarSearch.tsx` |
| `HomeSuggestionCards.tsx` | 无 import | `HomeWorkbenchPanel` |
| `DeliveryReadinessCard.tsx` | 无 import | `HomeInsightCards` |
| `ProjectKnowledgePanel.tsx` | 无 import | `ProjectKnowledgeConfigPanel` / `ProjectKnowledgeSummary` |
| `ModelCapabilityTabs.tsx` | 无 import | `ModelManagerPanel` |

### 5.2 Rust（保留观察）

| 模块 | 判定 |
|------|------|
| `crates/onboard/` | 孤儿 crate；CLI 无入口 |
| `cli/bootstrap/browser_mcp.rs` | feature `tools-mcp` 使用 |
| `llm/whisper_model_fetch.rs` | feature `stt-local` 使用 |
| `#![allow(dead_code)]` 模块 | `slash_commands.rs`、`repl/inline.rs` 等 — 勿批量删 |

---

## 6. 收敛矩阵（摘要）

```text
Keep     : run, stream REPL, dashboard, desktop sidecar, WeChat+TG+Discord, cron, skills, harness core
Hide     : Tier 2 SSO/OAuth write/Browser gates 产品承诺
Defer    : Harness M1–M8 余项, AskUserQuestion 通道 parity, cron UI CRUD, onboard crate
Remove   : 孤儿 UI 组件（§5.1）；误导性「Tauri 未实现」文档表述（已修正）
Document : RemoteTrigger/Team partial, placeholder providers, Slack 只读
```

---

## 7. 验证命令

与 [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) 一致：

```bash
cargo fmt --all -- --check
ANYCODE_BUILD_DASHBOARD_UI=1 ./scripts/build-dashboard-ui.sh
cargo clippy --workspace --all-targets
cargo test --workspace
cargo test -p anycode-tools --features tools-lsp
cargo test -p anycode-tools --features tools-mcp
cargo check -p anycode-tools --features knowledge-embeddings
cd crates/dashboard-ui && npm test && npm run build
cargo build --release -p anycode
```

---

## 8. 后续 PR 拆分建议

1. `docs/audits` + 文档叙事修正（本批）
2. `warnings cleanup` — import + cfg_attr + rename
3. `dead references cleanup` — 孤儿 UI 删除
4. `onboard crate` — 单独 ADR/决议后再动
