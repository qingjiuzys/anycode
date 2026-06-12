---
title: Discord
description: Connect anyCode to Discord with a bot token and channel ID.
summary: Create a bot, invite it, send a test message, and run the bridge.
read_when:
  - You want to chat with anyCode from Discord.
  - You need channel ID or permission help.
---

# Discord

Use a Discord bot to send tasks from a server channel and execute them with anyCode locally.

After this page, you will know:

- how to create a bot in the Developer Portal
- how to invite the bot and copy a channel ID
- how to verify with a test message and start the bridge

## Quick path (Workbench)

1. Open **Settings → Channels** (or Setup **Channels** → Discord).
2. Create an application and bot in the [Developer Portal](https://discord.com/developers/applications).
3. Paste the bot token → **Verify connection**.
4. Open the **invite link** from the UI; grant **View Channel** and **Send Messages**.
5. Enable **Developer Mode** in Discord (Settings → Advanced), right-click your channel → **Copy Channel ID**.
6. Paste channel ID → **Send test message** → **Save**.
7. Run (keep terminal open):

```bash
anycode channel discord
```

Credentials: `~/.anycode/channels/discord.json`.

## CLI path

```bash
anycode setup --channel discord
anycode channel discord
```

## Privileged intents

Under **Bot → Privileged Gateway Intents**, enable **Message Content Intent** if you need full message text (recommended for anyCode).

## Permissions

The bot needs at least:

- View Channel
- Send Messages
- Read Message History (helpful for context)

Use the Workbench invite link or create an OAuth2 URL with those permissions.

## Troubleshooting

| Error | Meaning |
|-------|---------|
| 401 Unauthorized | Invalid or revoked bot token |
| 403 Forbidden | Bot not in server, or missing Send Messages / View Channel |
| 404 Not Found | Wrong channel ID |

Always **Send test message** in the Workbench before saving.

## Next

- [Telegram](./telegram) — Telegram bot setup  
- [WeChat & setup](./wechat) — WeChat bridge  
- [Notifications](./notifications) — routing events to channels
