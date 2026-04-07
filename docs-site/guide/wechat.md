---
title: WeChat & setup
description: First-time setup flow and the optional WeChat iLink bridge for anyCode.
summary: setup sequence, channel selection, and how the bridge shares the same agent runtime.
read_when:
  - You want phone-driven tasks alongside the CLI.
  - You are setting up WeChat after a headless install.
---

# WeChat & setup

## `setup`

One-shot first setup:

1. Ensures **user workspace** layout under **`~/.anycode/workspace`** (alongside WeChat data under **`~/.anycode/wechat`**).
2. Runs the **config wizard** if API settings are missing or invalid.
3. On a TTY, asks you to choose a channel (`wechat` / `telegram` / `discord`), then runs the selected setup flow.

```bash
anycode setup
anycode setup --channel wechat
```

Global flags **`--debug`**, **`-c/--config`**, and env vars such as **`WCC_DATA_DIR`** follow the same rules as **`anycode channel wechat`**.

## `channel wechat`

Use when you need to bind or re-bind WeChat:

```bash
anycode channel wechat
```

Requires an environment where **QR login** can complete (browser / GUI). For narrative detail in Chinese, see the historical **[简体中文 CLI](/zh/guide/cli)** page until localized copy fully moves here.

## Workspace default

The user-level **workspace root** registers recent working directories when you use TUI, **`repl`**, or **`run`**. WeChat **`workingDirectory`** in `config.env` defaults to this root when unset so daemon/LaunchAgent contexts with `cwd=/` still have a sane project root. Change per-task directory via WeChat **`/cwd`** when needed.

## Next

- [CLI sessions](./cli-sessions) — TUI, REPL, `run`  
- [Troubleshooting](./troubleshooting) — no TTY / QR issues  
