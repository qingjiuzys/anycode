# 文档位置说明

面向用户的正文以 **VitePress 文档站**（中英双语）为准；本目录保留**维护者文档**、ADR 与按主题归类的运营材料。

| 受众 | 入口 |
|------|------|
| 用户 | [`docs-site/guide/`](../docs-site/guide/) · 本地 `cd docs-site && npm run dev` |
| 维护者 backlog | **[`roadmap.md`](roadmap.md)**（SSOT，含 §3.5 **0.3** 计划） |
| 架构 | [`architecture.md`](architecture.md) · ADR [`adr/`](adr/) |
| 文档地图 | 本文件 |

GitHub Pages：见 [`.github/workflows/docs.yml`](../.github/workflows/docs.yml)。

---

## 目录结构（2026-06 整理后）

```text
docs/
 README.md          ← 本文件
 roadmap.md         ← 执行层 backlog / 0.3 SSOT
 architecture.md

 adr/               ← 架构决策（不参与 VitePress 构建）

 planning/          ← 计划、验收、Harness（0.4 技术 hardening）
 comparisons/       ← OpenClaw / Claude / WorkBuddy / 微信对标
 ops/               ← Cron、MCP、工具治理、终端、通道运维备忘
 workbench/         ← Digital Workbench 状态、API、0.3 网页控制台规划
 archive/           ← 历史 sprint / 旧 Workbench 里程碑（只读）
 references/        ← 外部实现对照备忘
 issue-drafts/      ← GitHub issue 草稿
```

**原则**

1. **迭代任务只改 [`roadmap.md`](roadmap.md)**（及 `adr/`）；不要在 `docs-site` 重复 now/next/later 列表。
2. **产品 MVP / 工具矩阵** 仍以 [`docs-site/guide/roadmap.md`](../docs-site/guide/roadmap.md) 为准。
3. **一次性 sprint 日志、QA 缺陷表、HTML 快照** 已删除；历史 Workbench 里程碑见 [`archive/workbench/`](archive/workbench/)。

---

## 当前入口（按主题）

### 规划与 0.3（网页控制台）

| 文档 | 用途 |
|------|------|
| [`roadmap.md`](roadmap.md) | now / next / later、§3.5 **0.3** 交付包（登录/订阅/企业壳）、决策表 |
| [`workbench/digital-workbench-next-steps-zh.md`](workbench/digital-workbench-next-steps-zh.md) | **0.3 规划入口** — 账号/套餐/用量/API/企业 |
| [`workbench/digital-workbench-api.md`](workbench/digital-workbench-api.md) | API 合约（含 auth 模式） |
| [`workbench/digital-workbench-permissions.md`](workbench/digital-workbench-permissions.md) | 角色与 enterprise 模式 |
| [`planning/closure-plan-2026-06.md`](planning/closure-plan-2026-06.md) | 2026-06 套件收口波次 |
| [`planning/production-harness-hardening.md`](planning/production-harness-hardening.md) | Tier 1.5 Harness M0–M8（**0.4**，非 0.3） |
| [`planning/eval-harness.md`](planning/eval-harness.md) | `anycode eval` / mock LLM |
| [`planning/release-readiness-2026-05.md`](planning/release-readiness-2026-05.md) | 发布验收 checklist |

### 对标参考

| 文档 | 用途 |
|------|------|
| [`comparisons/openclaw-sync-brief-2026-05.md`](comparisons/openclaw-sync-brief-2026-05.md) | OpenClaw 差距矩阵 |
| [`comparisons/claude-reference-brief-2026-06.md`](comparisons/claude-reference-brief-2026-06.md) | Claude TS / Rust 参考 |
| [`comparisons/workbuddy-comparison-2026-06.md`](comparisons/workbuddy-comparison-2026-06.md) | WorkBuddy 七域矩阵 |
| [`comparisons/weixin-plugin-parity.md`](comparisons/weixin-plugin-parity.md) | 微信 npm 插件 vs Rust 桥 |

### 运维与实现（0.4 / Epic A–G）

| 文档 | 用途 |
|------|------|
| [`ops/tool-governance.md`](ops/tool-governance.md) | 工具审计 / 权限 |
| [`ops/mcp-stdio-lifecycle.md`](ops/mcp-stdio-lifecycle.md) | MCP stdio 生命周期 |
| [`ops/mcp-controlled-reconnect.md`](ops/mcp-controlled-reconnect.md) | ADR 007 摘要 |
| [`ops/cron-observability.md`](ops/cron-observability.md) | `cron-runs.jsonl` |
| [`ops/cron-production.md`](ops/cron-production.md) | Cron 生产语义 |
| [`ops/channel-production.md`](ops/channel-production.md) | IM 通道运维 |
| [`ops/stream-repl-layout.md`](ops/stream-repl-layout.md) | 流式 REPL 布局 |
| [`ops/term-smoothness-baseline.md`](ops/term-smoothness-baseline.md) | 终端观感基线 |
| [`ops/terminal-load-model.md`](ops/terminal-load-model.md) | transcript 负载模型 |

### Digital Workbench

| 文档 | 用途 |
|------|------|
| [`workbench/digital-workbench-STATUS.md`](workbench/digital-workbench-STATUS.md) | 一页状态 |
| [`workbench/digital-workbench-next-steps-zh.md`](workbench/digital-workbench-next-steps-zh.md) | 后续规划（中文） |
| [`workbench/digital-workbench-api.md`](workbench/digital-workbench-api.md) | API 合约 |
| [`workbench/workbench-ipc.md`](workbench/workbench-ipc.md) | CLI ↔ Dashboard IPC |
| 用户指南 | [`docs-site/guide/dashboard.md`](../docs-site/guide/dashboard.md) |

仓库根 [`WORKBENCH.md`](../WORKBENCH.md) 为 Workbench 快捷入口。

---

## ADR 索引

| ADR | 主题 |
|-----|------|
| [000](adr/000-runtime-orchestration.md) | `AgentRuntime` 编排权威 |
| [001](adr/001-memory-pipeline-and-store.md) | Memory pipeline / store |
| [002](adr/002-cli-composition-root.md) | CLI bootstrap 组合根 |
| [003](adr/003-http-daemon-deprecated.md) | 不恢复 HTTP daemon |
| [004](adr/004-session-rewind.md) | 会话 rewind（Proposed） |
| [005](adr/005-repl-clear-vs-transcript.md) | `/clear` vs transcript（Proposed） |
| [006](adr/006-transcript-virtual-scroll-rfc.md) | 虚拟滚动 RFC（Proposed） |
| [007](adr/007-mcp-session-reconnect-policy.md) | MCP 受控重连政策 |
| [008](adr/008-channel-ask-user-question-phasing.md) | 通道 AskUserQuestion |
| [009](adr/009-graph-memory-spike.md) | Graph memory spike |
| [010](adr/010-cooperative-cancel-and-nested-agents.md) | 协作取消 / 嵌套 agent |

---

## 已删除 / 归档说明

**已删除**（内容已被 SSOT 覆盖或过期）：`implementation-audit-checklist.md`、`autonomous-8h-*`、`qa-defect-log.md`、`qa-manual-llm-checklist.md`、`customer-feedback-issues-2026-06.html`、`flutter-app-template.md`、`work-run/`、`google-api-test-results-2026-05-29.md`。

**已归档**：`archive/sprints/`（production-convergence、autonomous-8h、旧 product-plan）、`archive/workbench/`（V1/V2 handoff、静态原型 HTML）。

若外链仍指向旧路径（如 `docs/cron-observability.md`），请改为 `docs/ops/cron-observability.md` 或文档站 URL。
