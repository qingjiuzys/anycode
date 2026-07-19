---
name: doc-summary
description: Summarize Markdown, text, or PDF-like attachments in batch.
description_zh: 批量摘要 Markdown、文本或 PDF 类附件。
name_zh: 文档摘要
category: business
version: 1.2.0
mode: instructions
approval: read-only-unless-writing-output
channel_capabilities: [files, markdown]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# doc-summary

> **中文**：批量总结 Markdown、文本或类 PDF 附件。  
> **English**: Summarize Markdown, text, or PDF-like attachments in batch.

## Use when

- One or more **local** Markdown/text documents need concise, comparable summaries.
- The user needs decisions, risks, facts, or action items extracted from documents.

Do not silently fetch remote URLs; use the web capability only when the user explicitly includes URLs.

## Preconditions (mandatory)

- Require at least one concrete file path or an attached document.
- If the user only says “summarize this task/session” with **no paths**, ask for paths
  (or summarize the chat clearly labeled as **conversation summary**, not a document summary).
- Never invent document contents or pretend unread files were summarized.

## Workflow

1. Confirm the document set and desired summary depth.
2. Read files with tools (`Read` / batch) in bounded chunks; preserve source path and title.
3. For each file output: **Purpose**, **Key points**, **Decisions**, **Risks**, **Actions**.
4. For multiple files, add a cross-document comparison and contradiction section.
5. Default artifact path when requested: `reports/document-summary.md`.

## Quality contract

- Distinguish direct facts from interpretation.
- Cite local source paths and page/section labels when available.
- Never claim an unreadable or unsupported file was summarized.

## Failure recovery

- List unreadable files separately and continue with readable inputs.
- For oversized inputs, summarize in batches and create a final synthesis.
