---
title: Quick start
description: Install anyCode, complete Workbench setup, and run your first chat in minutes.
---

# Quick start

> **Enterprise-grade, free & open-source Agent** — **self-host** on your machine or private network. BYOK models; data stays local by default.

![anyCode enterprise Agent workbench — self-hosted](/docs/assets/screenshots/home.png)
*Local Digital Workbench — projects, sessions, deliverables, and approvals on your hardware*

For **first-time users**. Follow these four steps to chat with the AI assistant on your machine.

## What you'll have

- anyCode installed (macOS desktop app recommended)
- Models configured in Workbench **Settings**
- A successful test message

## Step 1: Install

On macOS, download **`anyCode_<version>_aarch64.dmg`** from [GitHub Releases](https://github.com/qingjiuzys/anycode/releases), drag **anyCode** into Applications, and open it.

Other platforms: [Install](./install).

## Step 2: Open the Workbench

Launch **anyCode** — the Digital Workbench opens inside the app window.

For headless installs (`anycode-daemon`), open **`http://127.0.0.1:43180`** in your browser.

![Workbench home](/docs/assets/screenshots/home.png)
*Home — start a session or pick up where you left off*

## Step 3: First-time setup

On first launch you'll see the **setup wizard** (`/setup`):

1. Pick a model provider and enter your API key (BYOK — keys stay on your machine)
2. Optionally enable memory and vector search

![Setup wizard](/docs/assets/screenshots/setup.png)
*Setup wizard — model and API key*

Settings are saved to **`~/.anycode/config.json`**. Change them anytime under **Settings**.

![Settings](/docs/assets/screenshots/settings.png)
*Settings — models, notifications, browser, Skills*

## Step 4: Send a test message

1. Click **New session** on the home page, or open a **Project**
2. Send:

   > Reply with only: OK

3. Expected: the assistant replies `OK`

If nothing comes back, see [Common issues](./troubleshooting).

## What's next

| Goal | Doc |
|------|-----|
| Learn each sidebar page | [Workbench tour](./workbench) |
| PDFs, spreadsheets, slides | [Conversation deliverables](./deliverables) |
| Scheduled tasks | [Scheduled reminders](./cli-scheduler) |
| Change models or keys | [Models](./models) |

## Where docs live

Use **Docs** in the Workbench sidebar, or visit [anycode.work/docs](https://anycode.work/docs/) (local dev: `http://127.0.0.1:43200/docs/`).

![Docs site](/docs/assets/screenshots/docs-portal.png)
*Online docs — same look as the product*

简体中文: [快速开始](/docs/zh/guide/getting-started).
