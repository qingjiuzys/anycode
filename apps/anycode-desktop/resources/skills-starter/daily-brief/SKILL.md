---
name: daily-brief
description: Fetch news or RSS topics and produce a Markdown daily brief.
description_zh: 抓取新闻或 RSS 主题，生成 Markdown 格式日报。
name_zh: 每日简报
category: business
version: 1.1.0
---

# daily-brief

> **中文**：抓取新闻/RSS 主题并生成 Markdown 日报。  
> **English**: Fetch news or RSS topics and produce a Markdown daily brief.

## Preconditions

- Prefer topics the user names; if none, pick a tight theme related to their workspace
  (e.g. AI agents / coding tools) and state that assumption in the brief.

## Workflow (mandatory)

1. Use **WebSearch** (and **WebFetch** when needed) to gather **recent** items.
2. Write Markdown with:
   - Title + date
   - **今日聚焦** (2–4 sentences)
   - **优先事项** (3–5 bullets)
   - **风险** (1–2 bullets)
   - **来源** — each factual item must include a source URL or “search title + publisher”
3. Default save path: workspace root `brief.md` (or the path the user specifies).
4. Do not auto-post to social media.

## Quality contract

- Do not fabricate headlines or upvotes counts.
- If search fails, say so and produce a short outline marked **offline draft**.
- Keep under ~40 lines unless the user asks for more depth.
