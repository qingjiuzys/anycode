---
title: Using anyCode in the terminal
description: Everyday scenarios—ask questions, edit code, run tasks—without a command cheat sheet.
---

# Using anyCode in the terminal

anyCode works in your **terminal**: chat-style questions, or multi-step work inside a project folder.

## What you can do

| Scenario | How | What you get |
|----------|-----|----------------|
| Quick question | Run `anycode` and type | An answer; file lookup when needed |
| Work in a project | `cd` to the project, then `anycode` | The assistant uses that folder as workspace |
| One-shot task | `anycode run "Summarize this week's changes"` | Multi-step work with output in terminal or files |
| Scheduled reminders | Workbench **Automations**, or [Scheduled reminders](./cli-scheduler) | Runs your instructions on a schedule |

## Three steps to start

1. **Install and run setup** — [Install](./install) and [Quick start](./getting-started).
2. **`cd` into your project** and run `anycode`.
3. **State the goal in one sentence**, e.g. “Shorten the install section in README.”

Follow the model wizard the first time; change models later in Settings.

## Interfaces (no jargon required)

- **Default** — full-screen chat for longer work.
- **Line-by-line** — `anycode repl` for minimal environments.
- **Fire and forget** — `anycode run "…"` for a single task.

When unsure, just run `anycode`.

## Approvals

Before editing files or running shell commands, you may be asked to allow or deny. This protects your machine:

- **Allow once** — only this action
- **Deny** — the assistant stops or tries another approach

Adjust security options under Workbench **Settings → Security** if needed.

## Workbench together

- Terminal runs show up under **Projects / Conversations**.
- Scheduled jobs are created and monitored under **Automations**.
- Reports are generated on the **Reports** page.

## Something wrong?

| Symptom | Try |
|---------|-----|
| Model errors | Re-run `anycode setup`; check API keys and network |
| Stuck waiting | Look for a pending approval; or `Ctrl+C` and retry |
| Wrong project context | Confirm `pwd` is your project root |

See [Common issues](./troubleshooting). For command details: [Run, REPL & TUI](./cli-sessions) under *Learn more*.

简体中文: [终端里怎么用](/zh/guide/cli).
