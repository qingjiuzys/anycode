---
name: cn-daily-brief
description: Produce a Chinese Markdown daily brief from news, RSS, or user topics.
description_zh: 根据新闻、RSS 或用户指定主题，生成中文 Markdown 日报。
name_zh: 中文日报
category: writing
version: 1.1.0
mode: instructions
approval: read-only-unless-writing-output
channel_capabilities: [files, markdown]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: true
---

# cn-daily-brief

> **中文**：抓取或整理当日信息，输出结构化中文日报（标题、3–5 条要点、来源链接）。
> **English**: Produce a Chinese Markdown daily brief with headlines, bullets, and source links.

## 适用场景 / When to use

**适用：**
- 用户要求生成中文日报、每日简报或当日信息汇总。
- 信息源为网络新闻、RSS、用户指定主题或本地文件。

**不适用：**
- 周报、月报（使用 cn-weekly-report）。
- 会议纪要（使用 cn-meeting-minutes）。
- 英文日报/状态更新（使用 internal-comms）。
- 自动发布到社交媒体。

## 执行步骤 / Workflow

1. 确认用户指定的主题或关键词；若用户未指定，根据工作区上下文推断并明确说明假设。
2. 使用 **WebSearch**（必要时 **WebFetch**）收集当日信息；若用户提供本地文件则优先 **Read**。
3. 输出 Markdown 日报，结构如下：
   - **今日概览** — 一句话总结
   - **要点** — 3–5 条，每条含简短说明
   - **来源** — 可点击链接或文件路径
   - **待办/跟进**（可选）— 用户若需要则列出
4. 默认在对话中输出；需要落盘时保存到 `brief/YYYY-MM-DD.md`。
5. 不自动发帖到社交媒体；仅生成文件或在对话中回复。

## 质量契约 / Quality contract

- 不捏造标题、数据或点赞数；每条事实性内容必须附来源 URL 或「搜索标题 + 发布方」。
- 语气简洁、商务中文，避免夸张标题。
- 搜索结果有限时如实说明，不凭空填充条目。
- 控制篇幅在约 40 行以内，除非用户明确要求更深入。

## 失败恢复 / Failure recovery

- 网络搜索失败时，说明情况并生成标注为 **离线草稿** 的简要大纲，不阻塞交付。
- 用户提供的本地文件不可读时，单独列出并继续处理可读内容。
- 信息源不足时，标注 `[信息待补充]` 并建议用户补充关键词或文件。
