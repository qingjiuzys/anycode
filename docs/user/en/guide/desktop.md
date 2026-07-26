---
title: Desktop app (macOS)
description: anyCode.app bundles the Digital Workbench, native speech, and on-device OCR.
---

# Desktop app (macOS)

**anyCode.app** is the recommended macOS experience. It embeds the Digital Workbench inside the app window (not as a separate browser tab), plus native capabilities not available in a browser-only session:

- **Apple Speech** — voice input without downloading Whisper
- **Apple Vision OCR** — on-device text extraction from images

## Install

1. Download **`anyCode_<version>_aarch64.dmg`** from [Releases](https://github.com/qingjiuzys/anycode/releases).
2. Open the disk image and drag **anyCode** to **Applications**.
3. Launch **anyCode** — Workbench opens in the app window.

::: tip
On macOS, use the **anyCode.app** window for Workbench. `http://127.0.0.1:43180` is API-only when the desktop app is running (no full UI in Safari/Chrome). Headless/Linux installs still use the browser at that URL.
:::

## First-time setup

Open **Settings** or visit **`/setup`** in the Workbench to configure your model (BYOK) and memory.

## Headless scheduler

Keep the desktop app running for Workbench chat and automations. For a dedicated scheduler process on a server, install **`anycode-daemon`** — see [Headless daemon](./daemon).

## Related

- [Open the Workbench](./dashboard)
- [Workbench tour](./workbench)
- [Getting started](./getting-started)

简体中文：[桌面应用（macOS）](/zh/guide/desktop).
