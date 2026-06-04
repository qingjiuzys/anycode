---
title: Scheduled reminders
description: Create jobs with plain language and monitor runs in the Workbench.
---

# Scheduled reminders

Have anyCode do something on a schedule—daily status summaries, recurring checks, and similar.

## Recommended: create in the Workbench

1. Open the [Workbench](./dashboard) → **Automations**.
2. Under **Create scheduled task**:
   - **Natural language schedule** — e.g. “every day at 8am”, then **Parse → cron**.
   - **What to do** — one clear sentence, e.g. “Remind me to review open PRs.”
3. Click **Create job**.

Check **Registered jobs** and **Recent triggers**; use **Retry now** on failures.

## Keep something running

Triggers need a local scheduler:

- With the **desktop app**, keep anyCode running.
- Terminal-only setups need the scheduler started as described in [Install](./install).

## Failure notifications (optional)

When creating a job, you can set a failure destination (e.g. WeChat or webhook).

## Something wrong?

| Symptom | Try |
|---------|-----|
| Never runs | Ensure scheduler/desktop app is up; check run history |
| Wrong time | Fix timezone; re-parse the schedule |
| Wrong output | Make the task description more specific; open the linked session |

简体中文: [定时提醒](/zh/guide/cli-scheduler).
