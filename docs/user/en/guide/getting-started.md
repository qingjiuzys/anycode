---
title: Getting started
description: Install anyCode, complete Workbench setup, and run your first task.
summary: A non-technical first-run path with clear steps and what to do if something fails.
read_when:
  - You are new to anyCode and want the shortest path to a working setup.
---

# Getting started

For first-time users who want to get usable quickly.

After this page, you will have:

- anyCode installed (desktop app or `anycode-daemon`)
- model configured via Workbench **`/setup`**
- one successful test chat

## Five-minute path

1. **Install** — [Install](./install): macOS **anyCode.app** (recommended) or Linux/Windows **`anycode-daemon`**.  
2. **Open Workbench** — launch the desktop app or visit `http://127.0.0.1:43180` after starting the embedded dashboard.  
3. **Complete `/setup`** — choose model, memory / embedding ([Memory notes](./memory)).  
4. **Verify** — send a short message in the Workbench composer.

## Requirements

- **Prebuilt install**: no Rust needed.
- **Source build only**: Rust + Cargo (`cargo build --release` or desktop crate).

## First-time setup (Workbench)

Open **`http://127.0.0.1:43180/setup`** (or **Settings** in the app) and follow the wizard:

1. Model / provider (BYOK)
2. Memory and optional embeddings

No terminal `Workbench /setup` command — configuration is shared via `~/.anycode/config.json`.

## Verify

In the Workbench home or a project conversation, send:

> Reply with OK only

Expected: assistant replies `OK`. Tool traces and approvals appear in the transcript.

## Choose your next experience

| Goal | What to do | Guide |
|------|------------|-------|
| **Daily use (macOS)** | **anyCode.app** — Workbench + Apple Speech / Vision OCR | [Desktop app](./desktop) |
| **Workbench** | Projects, sessions, assets, security inbox | [Workbench tour](./workbench) |
| **Scheduled jobs** | Workbench **Automations** + `anycode-daemon scheduler` | [Scheduled reminders](./cli-scheduler) |
| **Headless server** | `anycode-daemon` for scheduler | [Headless daemon](./daemon) |
| **Switch models / BYOK** | Workbench **Settings** | [Models & endpoints](./models) |
| **Integrate / extend** | Workbench REST API, API tokens, Skills, MCP | [Architecture](./architecture) |

## If something fails

- Workbench won't load → confirm desktop app is running or dashboard is listening on port **43180**
- `anycode-daemon: command not found` → check PATH in [Install](./install)

## UI language

Set in Workbench **Settings**, or:

```bash
export ANYCODE_LANG=zh
# or
export ANYCODE_LANG=en
```

## Next

- [Install](./install)
- [Models & endpoints](./models)
- [Digital Workbench](./workbench)
- [Scheduled reminders](./cli-scheduler)
- [Troubleshooting](./troubleshooting)

简体中文：[快速开始](/zh/guide/getting-started).
