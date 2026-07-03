---
title: Headless daemon
description: anycode-daemon runs channel bridges and the cron scheduler without the desktop app.
---

# Headless daemon (`anycode-daemon`)

The standalone **`anycode-daemon`** binary replaces the retired terminal `anycode` CLI for long-running services:

| Subcommand | Purpose |
|------------|---------|
| `scheduler` | Cron / automations trigger loop |
| `wechat-bridge` | Personal WeChat iLink bridge |
| `telegram-bridge` | Telegram bot bridge |
| `discord-bridge` | Discord bot bridge |

Install via [Install](./install) (Linux/Windows tarball or `cargo install` from `crates/channel-bridge`).

## Examples

```bash
anycode-daemon scheduler
anycode-daemon wechat-bridge
anycode-daemon telegram-bridge
anycode-daemon discord-bridge
```

Configuration lives in `~/.anycode/config.json` (same schema as the Workbench). Complete first-time model setup in the Workbench **`/setup`** wizard — desktop app or embedded dashboard.

## Desktop vs daemon

| Scenario | Use |
|----------|-----|
| macOS daily use | **anyCode.app** (Workbench + native STT/OCR) |
| Linux server / NAS | **`anycode-daemon`** for channels + scheduler |
| Automations only | `anycode-daemon scheduler` (or keep desktop app running) |

The old HTTP `anycode daemon` subcommand (POST `/v1/tasks`) was removed — see [ADR 003](https://github.com/qingjiuzys/anycode/blob/main/docs/adr/003-http-daemon-deprecated.md).

## Related

- [Scheduled reminders](./cli-scheduler)
- [WeChat & setup](./wechat)
- [Install](./install)

简体中文：[无头守护进程](/zh/guide/daemon).
