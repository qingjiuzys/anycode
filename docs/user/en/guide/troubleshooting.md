---
title: Common issues
description: Install, Workbench, models, and scheduled jobs — FAQ.
---

# Common issues

## Install & first launch

**Workbench won't open?**  
Ensure **anyCode** is running. Browser users: `http://127.0.0.1:43180`. If the port is busy, stop the other process using 43180.

**Setup wizard (/setup) rejects API key?**  
Verify the key and network access to your provider. Re-save under **Settings → Models**.

## Workbench UI

**Empty project/session lists?**  
Complete [Quick start](./getting-started), then add a **Project** or **New session**.

**UI in English?**  
Switch language to 中文 in the top bar.

**Broken layout?**  
Hard refresh (Cmd+Shift+R) or restart anyCode.

## Chat & models

**Message spins forever?**  
Check network; verify model and API key in **Settings**; check provider quota.

**Approval before every edit?**  
By design. Adjust policy under **Settings → Security** (don't disable all checks blindly).

**Switch models?**  
**Settings → Models** — pick another provider or model ID.

## Scheduled jobs

**Job never runs?**  
anyCode or `anycode-daemon` must stay running. Check **Automations → Run history**.

**Failed run?**  
Click **Retry**; open the linked session for errors; make the task description more specific.

## Deliverables

**File on disk but no chat card?**  
Refresh the session; see [Deliverables](./deliverables).

## macOS desktop

**Blank window on launch?**  
Wait a few seconds for the embedded server; quit and reopen if needed.

**Blurry Dock icon?**  
Use the latest Release; restart Dock: `killall Dock`

---

Still stuck? [GitHub Issues](https://github.com/qingjiuzys/anycode/issues) — include OS version, anyCode version ( **Settings → About** or Release tag), steps, and screenshots.

简体中文: [常见问题](/docs/zh/guide/troubleshooting).
