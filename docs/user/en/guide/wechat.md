---
title: WeChat & setup
description: First-time setup flow and the optional WeChat iLink bridge for anyCode.
summary: Choose the right command quickly, then bind WeChat with minimum steps.
read_when:
  - You want phone-driven tasks alongside the CLI.
  - You are setting up WeChat after a headless install.
---

# WeChat & setup

For users who want to send requests from WeChat (mobile) and execute with anyCode.

After this page, you will know:

- which command to run first
- how to bind WeChat quickly
- what to check when QR or working directory fails

## Which command should I use?

- First-time model setup → Workbench **`/setup`** or **Settings**
- Bind/re-bind WeChat only → `anycode-daemon wechat-bridge`
- Configure Telegram/Discord → Workbench **Settings → Channels**, then `anycode-daemon telegram-bridge` or `discord-bridge`

## Workbench setup

Complete model and provider configuration in the Workbench wizard:

1. Open `http://127.0.0.1:43180/setup` (desktop app or embedded dashboard)
2. Configure model/provider when needed
3. Optional: note channel preferences (bridges read `~/.anycode/config.json`)

## `wechat-bridge`

Run this when:

- you skipped WeChat during setup
- you changed machine/account and need to bind again

```bash
anycode-daemon wechat-bridge
```

Expected output: QR binding flow starts.

Needs a machine that can complete QR login (browser/GUI).

## Common issue

If tasks run in the wrong project folder, set project directory in WeChat with `/cwd`.
Expected output: following tasks run in the selected project directory.

## Agent outbound files / images / videos

After a task completes, the bridge can deliver artifacts to WeChat (parity with OpenClaw `openclaw-weixin` outbound media):

- **Triggers**: tool-written files (`FileWrite` / `Edit` artifacts), or file paths in the final reply
- **Types**: documents (pdf, docx, zip…), images (png, jpg…), videos (mp4, mov…)
- **Small text**: `.md` / `.txt` ≤24KB may be sent inline as text
- **Size limit**: CDN upload ≤10MB per file; larger files get a local-path note only
- **Voice**: inbound voice is transcribed; outbound voice messages are not supported yet

Example: ask the agent to “create report.pdf and send it to me”. Include the absolute path in the reply, or rely on tool artifacts even if the path is omitted.

## Advanced notes

- WeChat data directory is usually `~/.anycode/wechat`
- Workspace fallback directory is `~/.anycode/workspace`
- Advanced flags and env (`--debug`, `-c/--config`, `WCC_DATA_DIR`) follow `anycode-daemon` defaults

## Next

- [CLI sessions](./cli-sessions) — TUI, REPL, `run`  
- [Troubleshooting](./troubleshooting) — no TTY / QR issues  
