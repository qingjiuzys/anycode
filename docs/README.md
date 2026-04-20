# 文档位置说明

正文已迁至 **VitePress 文档站**（中英双语入口）：

- **源码**：仓库根目录 [`docs-site/`](../docs-site/)
- **本地预览**：`cd docs-site && npm install && npm run dev`
- **中文**：`/zh/guide/` 下各页（由 `docs-site/zh/guide/*.md` 构建）
- **English**：`/guide/` 下各页（`docs-site/guide/*.md`）

GitHub Actions 工作流 [`.github/workflows/docs.yml`](../.github/workflows/docs.yml) 在推送 `main` 时构建并发布到 **GitHub Pages**（需在仓库 Settings → Pages 中选择 GitHub Actions 源）。

面向用户的旧路径 `docs/*.md` 已移除，避免与 `docs-site` 双份漂移。若外链仍指向 `docs/cli.md`，请更新为文档站 URL 或 `docs-site/zh/guide/cli.md`。

## 架构决策记录（ADR）

设计与边界类决策放在 [`adr/`](adr/)（Markdown，不参与 VitePress 构建）。当前条目：

- [`000-runtime-orchestration.md`](adr/000-runtime-orchestration.md) — **`AgentRuntime` 为编排权威**，`Agent::execute` 非 CLI/TUI 主路径。
- [`001-memory-pipeline-and-store.md`](adr/001-memory-pipeline-and-store.md) — **`MemoryStore` 与 `pipeline` 后端**的关系与组合方式。
- [`002-cli-composition-root.md`](adr/002-cli-composition-root.md) — **CLI `bootstrap` 组合根**边界与依赖方向。
- [`003-http-daemon-deprecated.md`](adr/003-http-daemon-deprecated.md) — **不恢复 HTTP `anycode daemon`**（已移除 `daemon_http`）。
- [`004-session-rewind.md`](adr/004-session-rewind.md) — **会话 rewind / 撤销展示**（Proposed，待填决策）。
- [`005-repl-clear-vs-transcript.md`](adr/005-repl-clear-vs-transcript.md) — **`/clear` 与纯文本 transcript**（Proposed，待填决策）。
- [`006-transcript-virtual-scroll-rfc.md`](adr/006-transcript-virtual-scroll-rfc.md) — **虚拟滚动复启 RFC**（Proposed，待填决策）。

## 开发备忘（非文档站）

- [`roadmap.md`](roadmap.md) — **维护者 backlog 单一事实来源**（now / next / later、决策与待决项）。
- [`issue-drafts/`](issue-drafts/) — GitHub issue 正文草稿（与 §3 主线对齐时可复制或 `gh issue create --body-file`）。
- [`tui-smoothness-baseline.md`](tui-smoothness-baseline.md) — TUI 观感迭代基线与终端矩阵清单（与 `ANYCODE_TUI_*` 环境变量对照）。
- [`stream-repl-layout.md`](stream-repl-layout.md) — **`anycode repl` 流式 TTY**：自上而下页面结构、宿主 scrollback 与 Inline 视口、Dock 栈与 Tokio/UI 线程数据流。
- [`implementation-audit-checklist.md`](implementation-audit-checklist.md) — 重定向至 [`roadmap.md`](roadmap.md)（勿重复编辑清单正文）。
