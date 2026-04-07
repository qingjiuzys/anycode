---
title: WeChat & setup
description: First-time setup flow and the optional WeChat iLink bridge for anyCode.
summary: Choose the right command quickly, then bind WeChat with minimum steps.
read_when:
  - You want phone-driven tasks alongside the CLI.
  - You are setting up WeChat after a headless install.
---

# WeChat & setup

For users who want to send requests from WeChat (mobile) and execute with anyCode.

After this page, you will know:

- which command to run first
- how to bind WeChat quickly
- what to check when QR or working directory fails

## Which command should I use?

- First time setup -> `anycode setup`
- Bind/re-bind WeChat only -> `anycode channel wechat`
- Configure Telegram/Discord instead -> `anycode setup --channel telegram|discord`

## `setup`

`setup` is the recommended first command:

1. Checks workspace folders
2. Configures model/provider when needed
3. Lets you choose channel (`wechat` / `telegram` / `discord`)

```bash
anycode setup
anycode setup --channel wechat
```

Expected output: setup guides you into model config then channel flow.

## `channel wechat`

Run this when:

- you skipped WeChat in setup
- you changed machine/account and need to bind again

```bash
anycode channel wechat
```

Expected output: QR binding flow starts.

Needs a machine that can complete QR login (browser/GUI).

## Common issue

If tasks run in the wrong project folder, set project directory in WeChat with `/cwd`.
Expected output: following tasks run in the selected project directory.

## Advanced notes

- WeChat data directory is usually `~/.anycode/wechat`
- Workspace fallback directory is `~/.anycode/workspace`
- Advanced flags and env (`--debug`, `-c/--config`, `WCC_DATA_DIR`) follow CLI defaults

## Next

- [CLI sessions](./cli-sessions) — TUI, REPL, `run`  
- [Troubleshooting](./troubleshooting) — no TTY / QR issues  
