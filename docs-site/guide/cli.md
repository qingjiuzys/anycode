---
title: Terminal CLI (removed)
description: The anycode terminal CLI was retired; use the Desktop app, Workbench, or anycode-daemon.
---

# Terminal CLI (removed)

The **`anycode`** terminal binary (REPL, TUI, `run`, `setup`, `dashboard` subcommands) is **no longer shipped**. Use one of these instead:

| Former workflow | Use now |
|-----------------|---------|
| Interactive chat in a project | [Digital Workbench](./workbench) — chat in the browser or **anyCode.app** |
| `Workbench /setup` | Workbench **`/setup`** or **Settings** |
| anyCode desktop or Workbench at http://127.0.0.1:43180 | Launch **anyCode.app** (macOS) or run the embedded dashboard from a dev build |
| `anycode channel *` | `anycode-daemon wechat-bridge` / `telegram-bridge` / `discord-bridge` — [Headless daemon](./daemon) |
| `anycode-daemon scheduler` | `anycode-daemon scheduler` — [Scheduled reminders](./cli-scheduler) |
| `anycode run` one-shot | Workbench conversation composer or REST API |

## Quick links

- [Getting started](./getting-started)
- [Desktop app (macOS)](./desktop)
- [Headless daemon](./daemon)
- [Workbench tour](./workbench)

简体中文：[终端 CLI（已移除）](/zh/guide/cli).
