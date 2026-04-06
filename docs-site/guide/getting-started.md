---
title: Getting started
description: Install anyCode, run onboard, and verify with run or TUI in about five minutes.
summary: Minimal path from binary to first task; links to install, CLI sessions, and docs map.
read_when:
  - You are new to anyCode and want the shortest path to a working setup.
---

# Getting started

anyCode is a **Rust terminal AI coding assistant**: fullscreen TUI, line-based REPL, multi-turn tool calls, optional HTTP daemon, and an optional **WeChat** bridge.

## Five-minute path

1. **Install** — see [Install](./install) (one-line script, Release tarball, or `cargo build`).  
2. **`onboard`** — workspace + config wizard + optional WeChat on a TTY.  
3. **Verify** — `run` once, or open the default TUI / `repl`.

## Requirements

- **Prebuilt binary**: no Rust required.  
- **Build from source**: Rust (stable) and Cargo; **`edition = "2021"`**.  
- **WeChat QR**: needs a GUI-capable environment (see [WeChat & onboard](./wechat)).

## First run

```bash
./target/release/anycode onboard
./target/release/anycode onboard --skip-wechat
```

## Verify

```bash
./target/release/anycode run --agent general-purpose "Reply with OK only"
./target/release/anycode
```

In the TUI try **`/help`**, **`/agents`**, **`/tools`**, **`/exit`**. For native scrolling use **`anycode repl`** — see [Run, REPL & TUI](./cli-sessions).

## Locale (UI language)

Resolved from **`ANYCODE_LANG`** / **`LANGUAGE`**, then **`LC_ALL`** / **`LC_MESSAGES`** / **`LANG`**, then OS locale:

```bash
export ANYCODE_LANG=zh   # or en
```

Model-facing prompts default to **English** for stability.

## Next

- [Docs directory](./docs-directory) — curated map of all guides  
- [CLI overview](./cli) — subcommands and deep links  
- [Models](./models) — providers and **`config.json`**  
- [Architecture](./architecture) — runtime layout  

简体中文：[快速开始](/zh/guide/getting-started).
