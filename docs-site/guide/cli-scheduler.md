---
title: Cron & scheduler
description: CronCreate persistence, anycode scheduler, and single-instance lock.
summary: orchestration.json, scheduler.lock, embedded WeChat scheduler vs standalone CLI.
read_when:
  - You want scheduled agent tasks similar to OpenClaw orchestration.
---

# Cron & built-in scheduler

## What exists

1. **`CronCreate` / `CronDelete` / `CronList`** (tools): persist cron rows under **`~/.anycode/tasks/orchestration.json`**. Expression format follows the **`cron`** crate (6 fields: `sec min hour day month weekday`; 5-field Unix-style is accepted with `0` seconds — see **`crates/cli/src/scheduler.rs`** `normalize_cron_schedule_expr`).
2. **`anycode scheduler`**: long-running CLI that reads the same JSON and fires each **`command`** as a one-shot agent task with the **`--directory`** working directory (`crates/cli/src/scheduler.rs`).

Saving a job **does not** run it unless a scheduler loop is active.

## Single-instance lock (`scheduler.lock`)

Only **one** scheduler loop should tick on a machine: **`~/.anycode/tasks/scheduler.lock`** (exclusive advisory lock).

- If **`anycode scheduler`** is already running, a second `anycode scheduler` exits quietly (log: lock busy).
- The **WeChat bridge** may **embed** a scheduler (`tokio::spawn` in `run_wechat_daemon`) so you do not need a separate `anycode scheduler` process on the same host **unless** you prefer isolation.

## Standalone `anycode scheduler` (optional)

Run in a terminal, **tmux**, **systemd user unit**, or **macOS LaunchAgent** — same binary as the CLI.

**Example (systemd user, Linux)** — adjust paths and `WorkingDirectory`:

```ini
[Unit]
Description=anyCode builtin cron scheduler
After=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/anycode scheduler -C /path/to/workspace --reload-secs 30
Restart=on-failure

[Install]
WantedBy=default.target
```

**Example (LaunchAgent plist fragment, macOS)** — run `anycode` from your install path:

```xml
<key>ProgramArguments</key>
<array>
  <string>/usr/local/bin/anycode</string>
  <string>scheduler</string>
  <string>-C</string>
  <string>/path/to/workspace</string>
  <string>--reload-secs</string>
  <string>30</string>
</array>
```

Do **not** start two schedulers on the same machine without understanding the lock: the second will no-op.

## Channel mode (WeChat / Telegram / Discord)

The **`workspace-assistant`** agent exposes **`CronCreate` / `CronDelete` / `CronList`** so users can register jobs from chat. Remind users that **execution** still requires the embedded WeChat scheduler **or** a separately started **`anycode scheduler`**, and that **only one** lock holder runs ticks.

Chinese: [定时任务与调度器](/zh/guide/cli-scheduler).
