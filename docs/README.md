# 文档位置说明

面向用户的正文以 **VitePress 文档站**（中英双语入口）为准；本目录保留维护者文档、ADR、路线图、Workbench 运营材料和归档资料：

- **源码**：仓库根目录 [`docs-site/`](../docs-site/)
- **本地预览**：`cd docs-site && npm install && npm run dev`
- **中文**：`/zh/guide/` 下各页（由 `docs-site/zh/guide/*.md` 构建）
- **English**：`/guide/` 下各页（`docs-site/guide/*.md`）

GitHub Actions 工作流 [`.github/workflows/docs.yml`](../.github/workflows/docs.yml) 在推送 `main` 时构建并发布到 **GitHub Pages**（需在仓库 Settings → Pages 中选择 GitHub Actions 源）。

若外链仍指向旧用户文档（例如 `docs/cli.md`），请更新为文档站 URL 或 `docs-site/zh/guide/cli.md`。维护者文档可以继续放在 `docs/`，但应在本文件登记。

## 架构决策记录（ADR）

设计与边界类决策放在 [`adr/`](adr/)（Markdown，不参与 VitePress 构建）。当前条目：

- [`000-runtime-orchestration.md`](adr/000-runtime-orchestration.md) — **`AgentRuntime` 为编排权威**，`Agent::execute` 非 CLI/TUI 主路径。
- [`001-memory-pipeline-and-store.md`](adr/001-memory-pipeline-and-store.md) — **`MemoryStore` 与 `pipeline` 后端**的关系与组合方式。
- [`002-cli-composition-root.md`](adr/002-cli-composition-root.md) — **CLI `bootstrap` 组合根**边界与依赖方向。
- [`003-http-daemon-deprecated.md`](adr/003-http-daemon-deprecated.md) — **不恢复 HTTP `anycode daemon`**（已移除 `daemon_http`）。
- [`004-session-rewind.md`](adr/004-session-rewind.md) — **会话 rewind / 撤销展示**（Proposed，待填决策）。
- [`005-repl-clear-vs-transcript.md`](adr/005-repl-clear-vs-transcript.md) — **`/clear` 与纯文本 transcript**（Proposed，待填决策）。
- [`006-transcript-virtual-scroll-rfc.md`](adr/006-transcript-virtual-scroll-rfc.md) — **虚拟滚动复启 RFC**（Proposed，待填决策）。
- [`007-mcp-session-reconnect-policy.md`](adr/007-mcp-session-reconnect-policy.md) — **MCP stdio 健康 / 快速失败 / 受控重连策略**（**Accepted**，政策；代码层自动重连仍待定）。
- [`008-channel-ask-user-question-phasing.md`](adr/008-channel-ask-user-question-phasing.md) — **IM 通道 AskUserQuestion** 分期实现（**Telegram MVP 已落地**；仍为 Proposed 以覆盖后续通道）。
- [`009-graph-memory-spike.md`](adr/009-graph-memory-spike.md) — **Graph memory** spike notes。
- [`010-cooperative-cancel-and-nested-agents.md`](adr/010-cooperative-cancel-and-nested-agents.md) — **主会话、turn 与嵌套 agent 协作取消**。

## 开发备忘（非文档站）

- [`roadmap.md`](roadmap.md) — **维护者 backlog 单一事实来源**（now / next / later、决策与待决项）。
- [`refactor-map.md`](refactor-map.md) — **可持续重构地图**：模块所有权、热点、命名规则和迁移顺序。
- [`production-harness-hardening.md`](production-harness-hardening.md) — Digital Workbench **Tier 1.5**：执行轨迹、运行时预算、轨迹评估、工具/MCP 治理、声明式 workflow 和记忆治理。
- [`workbench-ipc.md`](workbench-ipc.md) — Digital Workbench 与 live CLI 之间的 approval、cancel、session、SSE 合约。
- [`digital-workbench-STATUS.md`](digital-workbench-STATUS.md) — Digital Workbench 当前状态和后续验收入口。
- [`digital-workbench-next-steps.md`](digital-workbench-next-steps.md) / [`digital-workbench-next-steps-zh.md`](digital-workbench-next-steps-zh.md) — Workbench 后续路线图。
- [`digital-workbench-api.md`](digital-workbench-api.md) / [`digital-workbench-deploy-production.md`](digital-workbench-deploy-production.md) / [`digital-workbench-permissions.md`](digital-workbench-permissions.md) — Workbench API、部署和权限说明。
- [`issue-drafts/`](issue-drafts/) — GitHub issue 正文草稿（与 §3 主线对齐时可复制或 `gh issue create --body-file`）。
- [`term-smoothness-baseline.md`](term-smoothness-baseline.md) — 流式终端观感迭代基线与终端矩阵清单（与 `ANYCODE_TERM_*` 等环境变量对照）。
- [`stream-repl-layout.md`](stream-repl-layout.md) — **`anycode repl` 流式 TTY**：自上而下页面结构、宿主 scrollback 与 Inline 视口、Dock 栈与 Tokio/UI 线程数据流。
- [`openclaw-sync-brief-2026-05.md`](openclaw-sync-brief-2026-05.md) — OpenClaw **2026.5.19** 对标矩阵（维护者）。
- [`weixin-plugin-parity.md`](weixin-plugin-parity.md) — 微信 npm 插件 vs Rust 桥差异表。
- [`cron-observability.md`](cron-observability.md) — 内置调度器 `cron-runs.jsonl` 字段说明。
- [`implementation-audit-checklist.md`](implementation-audit-checklist.md) — 重定向至 [`roadmap.md`](roadmap.md)（勿重复编辑清单正文）。
