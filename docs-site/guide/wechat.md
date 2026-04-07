---
title: WeChat & setup
description: First-time setup flow and the optional WeChat iLink bridge for anyCode.
summary: setup sequence, skip-wechat, and how the bridge shares the same agent runtime.
read_when:
  - You want phone-driven tasks alongside the CLI.
  - You are setting up WeChat after a headless install.
---

# WeChat & setup

## `setup`

One-shot first setup:

1. Ensures **user workspace** layout under **`~/.anycode/workspace`** (alongside WeChat data under **`~/.anycode/wechat`**).
2. Runs the **config wizard** if API settings are missing or invalid.
3. On a TTY, optionally starts the **WeChat bind** flow and login autostart bridge (same as `anycode wechat`).

```bash
anycode setup
anycode setup --skip-wechat
```

Global flags **`--debug`**, **`-c/--config`**, and env vars such as **`WCC_DATA_DIR`** follow the same rules as **`anycode wechat`**.

## `wechat`

Use when you skipped WeChat during setup or need to re-bind:

```bash
anycode wechat
```

Requires an environment where **QR login** can complete (browser / GUI). For narrative detail in Chinese, see the historical **[简体中文 CLI](/zh/guide/cli)** page until localized copy fully moves here.

## Workspace default

The user-level **workspace root** registers recent working directories when you use TUI, **`repl`**, or **`run`**. WeChat **`workingDirectory`** in `config.env` defaults to this root when unset so daemon/LaunchAgent contexts with `cwd=/` still have a sane project root. Change per-task directory via WeChat **`/cwd`** when needed.

## Next

- [CLI sessions](./cli-sessions) — TUI, REPL, `run`  
- [Troubleshooting](./troubleshooting) — no TTY / QR issues  
