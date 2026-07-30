---
title: Desktop app (macOS)
description: anyCode.app with embedded Workbench, native speech, and on-device OCR.
---

# Desktop app (macOS)

**anyCode.app** is the recommended way to use anyCode on macOS.

## vs browser-only

| Capability | Desktop app | Browser on :43180 only |
|------------|-------------|-------------------------|
| Full Workbench UI | ✅ In app window | ❌ API only |
| Apple speech input | ✅ | ❌ |
| On-device OCR (Vision) | ✅ | ❌ |
| Built-in browser (agent automation) | ✅ | Depends on build |

## Install

1. [GitHub Releases](https://github.com/qingjiuzys/anycode/releases) → **`anyCode_<version>_aarch64.dmg`**
2. Drag into Applications
3. Open **anyCode**

![Workbench in app](/docs/assets/screenshots/home.png)
*Workbench inside the desktop app*

## First-time setup

First launch opens the **setup wizard** (`/setup`). Add your model API key and memory options.

See [Quick start](./getting-started).

## Keep it running (scheduled jobs)

For [Scheduled reminders](./cli-scheduler), leave anyCode running in the background, or use **`anycode-daemon`** on a server.

## Related

- [Open the Workbench](./dashboard)
- [Workbench tour](./workbench)
- [Common issues](./troubleshooting)

简体中文: [桌面应用](/docs/zh/guide/desktop).
