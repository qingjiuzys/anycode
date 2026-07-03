---
title: 终端 CLI（已移除）
description: anycode 终端 CLI 已退役；请使用桌面应用、工作台或 anycode-daemon。
---

# 终端 CLI（已移除）

**`anycode`** 终端二进制（REPL、TUI、`run`、`setup`、`dashboard` 子命令）**已不再发布**。请改用：

| 原工作流 | 现方案 |
|----------|--------|
| 项目内交互对话 | [Digital Workbench](./workbench) 或 **anyCode.app** |
| `Workbench /setup` | Workbench **`/setup`** 或**设置** |
| anyCode desktop or Workbench at http://127.0.0.1:43180 | 启动 **anyCode.app**（macOS） |
| `anycode channel *` | `anycode-daemon wechat-bridge` 等 — [无头守护进程](./daemon) |
| `anycode-daemon scheduler` | `anycode-daemon scheduler` — [定时提醒](./cli-scheduler) |
| `anycode run` 单次任务 | Workbench 会话或 REST API |

## 相关

- [快速开始](./getting-started)
- [桌面应用](./desktop)
- [无头守护进程](./daemon)

English: [Terminal CLI (removed)](/guide/cli).
