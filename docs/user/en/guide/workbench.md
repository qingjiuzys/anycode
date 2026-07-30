---
title: Workbench tour
description: Sidebar pages for projects, sessions, automations, and deliverables.
---

# Workbench tour

The Workbench answers: **Which projects are active? How are chats going? Did scheduled jobs fail?** — all locally, no terminal required.

![Workbench home](/docs/assets/screenshots/home.png)
*Home — shortcuts and recent activity*

## Sidebar at a glance

| Page | What you see | Typical actions |
|------|----------------|-----------------|
| **Home** | Shortcuts, recent sessions | New session, resume work |
| **Projects** | Registered workspace folders | Add project, open details |
| **Sessions** | Chats grouped by project | Open a conversation |
| **Colleagues** | Other anyCode instances on LAN | Discover peers, hand off sessions |
| **Automations** | Cron jobs and run history | Create schedule, retry failures |
| **Artifacts** | Generated file index | Open PDFs, spreadsheets, decks |
| **Settings** | Models, Skills, notifications | API keys, install skill packs |

## Projects

Register a local folder so the assistant can read and write files there.

![Projects](/docs/assets/screenshots/projects.png)
*Projects — bind workspace roots*

## Settings

Models, language, built-in browser, MCP servers — all under **Settings**.

![Settings](/docs/assets/screenshots/settings.png)
*Settings — models and preferences*

## Automations

Describe *when* and *what* in plain language, e.g. “Every weekday at 9am, summarize yesterday's git commits.”

![Automations](/docs/assets/screenshots/automations.png)
*Automations — schedules and run log*

Keep **anyCode** or **`anycode-daemon`** running for jobs to fire on time.

## Colleagues (session handoff)

Discover other anyCode instances on your network and hand a session to another machine.

![Colleagues](/docs/assets/screenshots/colleagues.png)
*Colleagues — discover and hand off*

## In conversations

- **Text & images** — type or paste images (Vision model required)
- **Deliverable cards** — PDFs, Office files, spreadsheets, mind maps → [Deliverables](./deliverables)
- **Approvals** — confirmations before sensitive file edits (adjust in Settings)

## Language & theme

Switch **中文 / English** and light/dark theme in the top bar. **Docs** and **Help** follow your language.

## Problems?

| Symptom | Try |
|---------|-----|
| Page won't load | Ensure anyCode is running; check port 43180 |
| Empty lists | Complete `/setup`, then add a project or session |
| Jobs never run | Keep app/daemon running; check Automations run log |

More: [Common issues](./troubleshooting).

简体中文: [工作台导览](/docs/zh/guide/workbench).
