---
title: Common issues
description: Install, Workbench, terminal, and scheduled jobs—quick fixes.
---

# Common issues

## Install & setup

**`anycode` not found after install?**  
Add the install directory to PATH or open a new terminal. From source, use the full path to `target/release/anycode`.

**`setup` failed?**  
Usually network or API keys. Verify provider credentials and run `anycode setup` again.

## Workbench

**Can’t open `http://127.0.0.1:43180`?**  
Run `anycode dashboard` first; check for port conflicts. Change port in Settings if needed.

**Empty project list?**  
The Workbench lists workspaces that have already run tasks. Run `anycode` or `anycode run` in a project folder, then refresh.

**Wrong language?**  
Switch **中文 / English** in the top bar. Sidebar doc/help links follow the UI locale.

## Terminal

**Assistant ignores project files?**  
Check `pwd` is the project root.

**Too many approval prompts?**  
By design for safety. Adjust policies in Settings; don’t disable all checks blindly.

**Slow or interrupted replies?**  
Check network and quotas; `Ctrl+C` stops the current turn.

## Scheduled jobs

**Job never fires?**  
Keep scheduler or desktop app running; check **Automations → Recent triggers**.

**Shows failed?**  
Use **Retry now**; open the session for details; clarify the task text.

## macOS desktop app

**Small Dock icon?**  
Use the latest DMG; if the icon is cached, run `killall Dock`.

**Blank window?**  
Wait a few seconds for the sidecar; or run `anycode dashboard` manually, then reopen the app.

---

Still stuck? Open a [GitHub Issue](https://github.com/qingjiuzys/anycode/issues) with OS version, `anycode --version`, steps, and expected behavior.

简体中文: [常见问题](/zh/guide/troubleshooting).
