---
name: weekly-report
description: Compile a weekly work summary from notes, git activity, or user-provided bullets.
description_zh: 根据笔记、Git 活动或用户要点，整理周报摘要。
name_zh: 周报生成
category: business
version: 1.1.0
mode: instructions
approval: read-only-unless-writing-output
channel_capabilities: [files, markdown]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# weekly-report

> **中文**：从笔记、Git 活动或用户要点汇编周报。  
> **English**: Compile a weekly work summary from notes, git activity, or user-provided bullets.

## Use when

- The user wants a weekly update from notes, commits, tasks, or chat excerpts.
- The result should be manager-ready Markdown with evidence-backed progress.

Do not use for daily news briefs or meeting minutes.

## Workflow

1. Identify the reporting period, audience, and available evidence.
2. Read only the relevant notes, task lists, and git history. Do not infer completed work from file presence alone.
3. Group evidence into **Summary**, **Wins**, **In progress**, **Blockers**, and **Next week**.
4. Mark uncertain owners, dates, and metrics as `TBD` instead of inventing them.
5. Return Markdown in chat unless the user requests a file; suggested path: `reports/weekly-report-YYYY-MM-DD.md`.

## Quality contract

- Every stated completion must be supported by the provided evidence.
- Keep items concise, outcome-oriented, and free of duplicated bullets.
- Preserve concrete numbers, dates, commit references, and owner names.

## Failure recovery

- With insufficient evidence, produce a clearly labeled draft plus a short missing-information list.
- If git is unavailable, continue from user-provided notes rather than failing the whole report.
