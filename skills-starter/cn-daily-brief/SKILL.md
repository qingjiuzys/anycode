---
name: cn-daily-brief
description: Produce a Chinese Markdown daily brief from news, RSS, or user topics.
description_zh: 根据新闻、RSS 或用户指定主题，生成中文 Markdown 日报。
category: business
---

# cn-daily-brief

> **中文**：抓取或整理当日信息，输出结构化中文日报（标题、3–5 条要点、来源链接）。  
> **English**: Produce a Chinese Markdown daily brief with headlines, bullets, and source links.

## 输出结构

1. **今日概览** — 一句话总结
2. **要点** — 3–5 条，每条含简短说明
3. **来源** — 可点击链接或文件路径
4. **待办/跟进**（可选）— 用户若需要则列出

## 规则

- 默认使用 **WebSearch** / **WebFetch** 收集信息；用户若提供本地文件则优先 **Read**。
- 不自动发帖到社交媒体；仅生成文件或在对话中回复。
- 语气：简洁、商务中文，避免夸张标题。
