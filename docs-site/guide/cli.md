---
title: CLI overview
description: anycode binary, global flags, and links to focused CLI guides.
summary: Entry point for subcommands; points to sessions, models, WeChat, diagnostics, and removed HTTP daemon note.
read_when:
  - You need a map of CLI documentation before diving into one subcommand.
---

# CLI overview

**Binary:** `anycode`.

## Global flags

- **`--debug`** — verbose logging.  
- **`-c/--config <PATH>`** — config file path; if the path is given and missing, the CLI exits with an error.  
- **`--model <ID>`** — long option only on the **default TUI** entry (no subcommand); avoids clashing with **`repl`’s `-m/--model`**.  
- **`--ignore-approval`** (aliases **`--ignore`**, typo **`--ingroe`**) — skip interactive approval **for this process only**; does not rewrite `config.json`.

Subcommands that read/write config (**`run`**, **`tui`**, **`repl`**, **`model`**, **`channel`**, etc.) honor **`-c`**.

```bash
anycode config
```

See [Config & security](./config-security) for **`security.*`**, memory fields, and env vars.

## Section guides

| Topic | Page |
|--------|------|
| **`run`**, **`repl`**, fullscreen TUI, task logs | [Run, REPL & TUI](./cli-sessions) |
| **HTTP `daemon`** (removed) | [HTTP daemon (removed)](./cli-daemon) |
| **`model`*** subcommands | [Model commands](./cli-model) |
| **`list-agents`**, **`list-tools`**, **`test-security`** | [Discovery & test-security](./cli-diagnostics) |
| **`setup`**, **`wechat`** | [WeChat & setup](./wechat) |
| **`enable` / `disable` / `status` / `mode`** | Feature flags & routing snapshot (see [Feature flags](./releases#runtime-feature-flags)) |
| **`workspace`** | Project registry & per-directory defaults (see [Routing](./routing)) |

Runtime feature names are defined in **`anycode_core::FeatureFlag`** (`skills`, `workflows`, `goal-mode`, `channel-mode`, `approval-v2`, `context-compression`, `workspace-profiles`).

## Build from source

```bash
cargo build --release
./target/release/anycode --help
```

MCP: build with **`--features tools-mcp`**; env **`ANYCODE_MCP_COMMAND`**, **`ANYCODE_MCP_SERVERS`**, etc.

## Locale

CLI UI language: **`ANYCODE_LANG`**, **`LANGUAGE`**, **`LC_*`**, **`LANG`**, then OS (see [Config & security](./config-security)).

Chinese pages mirror this structure under **`/zh/guide/`**.
