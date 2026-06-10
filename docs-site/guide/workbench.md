---
title: Digital Workbench tour
description: Product guide to sidebar pages—projects, sessions, scheduled jobs, and reports.
---

# Digital Workbench tour

The Workbench is anyCode’s **local web dashboard**: see which projects are active, whether runs passed checks, and if scheduled jobs failed—without memorizing CLI flags.

## What you can do

- Browse registered projects and sessions
- Inspect run status, gates, and output files
- Create scheduled jobs and retry failures
- Generate project or session reports (Markdown or HTML)

## How to open it

**Browser:**

```bash
anycode dashboard --open
```

Default URL: `http://127.0.0.1:43180`

**macOS app:** Install the platform build from [Releases](https://github.com/qingjiuzys/anycode/releases) (macOS `.dmg`, Windows `.msi`/`.exe`, Linux `.deb`/`.AppImage`). The app starts the Workbench automatically.

## Conversations & vision

- **Web chat:** attach images in project conversations (requires a vision-capable model).
- **Channels:** WeChat, Telegram, and Discord accept images; Telegram/Discord voice messages are transcribed when `models.speech.stt` is configured.
- See [run-flow.md](https://github.com/qingjiuzys/anycode/blob/main/docs/run-flow.md) in the repo.

## Project knowledge

On a project page, configure **knowledge paths** and **reindex**. Desktop releases include vector search; dev CLI builds need the `knowledge-embeddings` feature. Doctor checks report index and vector status.

## MCP & browser

- **Settings → MCP servers:** edit `mcp.servers` in `~/.anycode/config.json` from the UI.
- **Settings → Notifications → Browser connector:** enable bundled Playwright MCP on Desktop; the home quick compose area also prompts when bundled.

## Sidebar pages

| Page | What you see | Typical actions |
|------|----------------|-----------------|
| **Overview** | Counts, running sessions, recent activity | Spot issues quickly |
| **Projects** | Workspaces, trust level, last activity | Open a project |
| **Conversations** | Sessions grouped by project | Open a thread timeline |
| **Automations** | Scheduled jobs, run history, guardrails | Create jobs, retry on failure |
| **Assets** | Files the assistant changed | Review outputs |
| **Reports** | Project/session reports | Export for sharing |
| **Audit** | Config change log | Trace policy edits |
| **Agents / Skills** | Roles and local skill packs | See installed skills |
| **Settings** | Login, models, notifications, ops | Change port, report format |

## Automations (scheduled jobs)

The Automations page is about **running tasks on a schedule**, not mixed with workflow session tables:

1. **Summary cards** — job count, recent failures, enabled guardrails
2. **Create job** — natural language schedule plus what the assistant should do
3. **Project guardrails** — e.g. block on gate failure, auto-report when done
4. **Jobs & run history** — see each trigger; retry failed runs

Keep the scheduler or desktop app running for jobs to fire.

## Language & theme

Use the top bar to switch **中文 / English** and light/dark theme. Sidebar **Documentation** and **Help** open the matching site locale.

## Something wrong?

| Symptom | Try |
|---------|-----|
| Page won’t load | Run `anycode dashboard`; check for port conflicts |
| Empty lists | Run a task in a project folder first, then refresh |
| Jobs never run | Ensure scheduler/desktop app is running; check run history |
| Stuck at login | On `127.0.0.1`, local user is usually trusted automatically |

More: [Common issues](./troubleshooting).

简体中文: [工作台导览](/zh/guide/workbench).
