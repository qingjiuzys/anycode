---
title: Channel AskUserQuestion (Telegram)
description: How AskUserQuestion works on the Telegram bridge with inline buttons and timeouts.
read_when:
  - You run anycode on Telegram and the model asks a multiple-choice question.
---

# Channel AskUserQuestion (Telegram)

When the agent calls the **`AskUserQuestion`** tool, the Telegram bridge sends an **inline keyboard** (tap-to-choose). Your tap is delivered as a **callback query** while the agent task is running.

## Behavior

- **One pending question per chat**: starting a new question replaces the previous pending request (same pattern as other channel brokers).
- **Serial runs per chat**: ordinary messages for the same chat are queued behind the in-flight task so the bridge can still **poll** for your button tap.
- **Timeout**: after several minutes without a tap, the question is discarded; the tool may return an error and the model can fall back to plain text.
- **Limits**: up to **8** options; **no** `multiSelect` on Telegram — keep `multiSelect` false.

## Fallback

If buttons fail, users may reply with a **digit** `1`–`N` matching the listed options; the system prompt for the Telegram channel reminds the model of this fallback.

## Maintainer references

- [ADR 008](https://github.com/qingjiuzys/anycode/blob/main/docs/adr/008-channel-ask-user-question-phasing.md) — phasing and scope  
- [Spike notes](https://github.com/qingjiuzys/anycode/blob/main/docs/ops/channel-ask-user-question-spike.md) — design sketch  
