---
title: Getting started
description: Install anyCode, run setup, and complete your first task in a few minutes.
summary: A non-technical first-run path with clear commands and what to do if something fails.
read_when:
  - You are new to anyCode and want the shortest path to a working setup.
---

# Getting started

For first-time users who want to get usable quickly.

After this page, you will have:

- anyCode installed
- `setup` completed
- one successful test command

## Five-minute path

1. **Install** — use [Install](./install).  
2. **Run setup** — choose model + channel (`wechat` / `telegram` / `discord`).  
3. **Verify** — run one command and check output.

## Requirements

- **Prebuilt install**: no Rust needed.
- **Source build only**: Rust + Cargo.
- **For WeChat QR login**: run setup on a machine that can open browser/GUI.

## First run (`setup`)

If `anycode` is already in PATH:

```bash
anycode setup
```

Expected output: setup wizard starts and asks for model/channel choices.  
Next step: complete setup, then run the Verify commands below.

If you are running directly from a local build output:

```bash
./target/release/anycode setup
```

Expected output: same setup wizard flow as `anycode setup`.  
Next step: after success, prefer using `anycode` if PATH is configured.

You can also choose channel explicitly:

```bash
anycode setup --channel wechat
anycode setup --channel telegram
anycode setup --channel discord
```

Expected output: setup skips channel menu and enters the selected channel flow.

## Verify

```bash
anycode run --agent general-purpose "Reply with OK only"
anycode
```

Expected output: first command prints `OK`; second command opens TUI.

In TUI you can try: `/help`, `/tools`, `/exit`.

## If something fails

- `anycode: command not found` -> check PATH in [Install](./install)
- `setup` cannot ask questions -> run in a real terminal (not CI/headless shell)
- WeChat QR cannot complete -> run `anycode channel wechat` on a GUI machine

## UI language

Set language quickly:

```bash
export ANYCODE_LANG=zh
# or
export ANYCODE_LANG=en
```

## Next

- [Install](./install)
- [WeChat & setup](./wechat)
- [Troubleshooting](./troubleshooting)
- [Docs directory](./docs-directory)

简体中文：[快速开始](/zh/guide/getting-started).
