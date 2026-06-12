---
title: Telegram
description: Connect anyCode to Telegram with a bot token and chat ID.
summary: Create a bot, verify the token, discover your chat, and run the bridge.
read_when:
  - You want to chat with anyCode from Telegram.
  - You need Chat ID help after creating a bot.
---

# Telegram

Use a Telegram bot to send tasks from your phone and run them with anyCode on your machine.

After this page, you will know:

- how to create a bot with BotFather
- how to verify token and find Chat ID
- how to start the bridge

## Quick path (Workbench)

1. Open **Settings → Channels** (or the Setup wizard **Channels** step).
2. Follow the guided steps: BotFather → paste token → **Verify connection**.
3. In Telegram, open your bot and send `/start`.
4. Click **Refresh chat list**, pick your chat, **Save**.
5. Run the bridge command shown in the UI (keep the terminal open):

```bash
anycode channel telegram
```

Credentials are stored at `~/.anycode/channels/telegram.json` (token stays local).

## CLI path

```bash
anycode setup --channel telegram
# or after credentials exist:
anycode channel telegram
```

## Create a bot

1. Open [@BotFather](https://t.me/BotFather) in Telegram.
2. Send `/newbot` and follow prompts.
3. Copy the **HTTP API token** (looks like `123456:ABC-DEF...`).

## Chat ID

Private chats: after you message the bot, the Workbench **Refresh chat list** shows your chat.

Manual fallback:

- Message the bot, then call Telegram `getUpdates` with your token, or
- Use a helper bot such as `@userinfobot` for your user id (private chats).

For **groups**, you may need BotFather `/setprivacy` → **Disable** so the bot receives messages.

## Troubleshooting

| Symptom | Fix |
|--------|-----|
| Verify fails | Token typo; create a new token in BotFather if leaked |
| Empty chat list | Send `/start` to the bot first, then refresh |
| Bot silent in group | Disable privacy mode; ensure bot is in the group |
| Bridge exits | Re-run `anycode channel telegram`; check model config |

## Next

- [Discord](./discord) — Discord bot setup  
- [WeChat & setup](./wechat) — WeChat iLink bridge  
- [Config & security](./config-security) — credential locations
