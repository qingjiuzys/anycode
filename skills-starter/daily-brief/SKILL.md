---
name: daily-brief
description: Fetch news or RSS topics and produce a Markdown daily brief.
description_zh: 抓取新闻或 RSS 主题，生成 Markdown 格式日报。
name_zh: 每日简报
category: business
version: 1.1.0
mode: instructions
approval: read-only-unless-writing-output
channel_capabilities: [files, markdown]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: true
---

# daily-brief

> **English**: Fetch news or RSS topics and produce a Markdown daily brief.
> **中文**：抓取新闻/RSS 主题并生成 Markdown 日报。

## When to use

**Use when:**
- The user asks for a daily news brief, morning summary, or topic digest.
- Sources are web news, RSS feeds, user-specified topics, or local files.

**Do not use when:**
- The user wants a Chinese daily brief (use cn-daily-brief).
- The user wants a weekly report (use cn-weekly-report).
- The task is to auto-post to social media.

## Preconditions

- Prefer topics the user names; if none, pick a tight theme related to their workspace (e.g. AI agents / coding tools) and state that assumption in the brief.

## Workflow

1. Confirm the user's topics or keywords; if unspecified, infer from workspace context and state the assumption.
2. Use **WebSearch** (and **WebFetch** when needed) to gather **recent** items.
3. Write Markdown with:
   - Title + date
   - **Focus of the day** (2–4 sentences)
   - **Priority items** (3–5 bullets)
   - **Risks** (1–2 bullets)
   - **Sources** — each factual item must include a source URL or "search title + publisher"
4. Default save path: workspace root `brief.md` (or the path the user specifies).
5. Do not auto-post to social media.

## Quality contract

- Do not fabricate headlines, upvote counts, or statistics.
- Every factual claim must include a source URL or "search title + publisher."
- Keep under ~40 lines unless the user asks for more depth.
- If search results are limited, say so honestly and do not pad with invented items.

## Failure recovery

- If search fails, say so and produce a short outline marked **offline draft**.
- If user-provided local files are unreadable, list them separately and continue with readable inputs.
- If information is insufficient, mark `[information pending]` and suggest the user provide additional keywords or files.
