# 文档位置说明

正文已迁至 **VitePress 文档站**（中英双语入口）：

- **源码**：仓库根目录 [`docs-site/`](../docs-site/)
- **本地预览**：`cd docs-site && npm install && npm run dev`
- **中文**：`/zh/guide/` 下各页（由 `docs-site/zh/guide/*.md` 构建）
- **English**：`/guide/` 下各页（`docs-site/guide/*.md`）

GitHub Actions 工作流 [`.github/workflows/docs.yml`](../.github/workflows/docs.yml) 在推送 `main` 时构建并发布到 **GitHub Pages**（需在仓库 Settings → Pages 中选择 GitHub Actions 源）。

旧路径 `docs/*.md` 已移除，避免与 `docs-site` 双份漂移。若外链仍指向 `docs/cli.md`，请更新为文档站 URL 或 `docs-site/zh/guide/cli.md`。

## 架构决策记录（ADR）

设计与边界类决策放在 [`adr/`](adr/)（Markdown，不参与 VitePress 构建）。当前条目：

- [`000-runtime-orchestration.md`](adr/000-runtime-orchestration.md) — **`AgentRuntime` 为编排权威**，`Agent::execute` 非 CLI/TUI 主路径。
