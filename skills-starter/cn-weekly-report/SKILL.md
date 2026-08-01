---
name: cn-weekly-report
description: Summarize a week's work into a Chinese weekly report for managers or teams.
description_zh: 将一周工作整理为面向团队或上级的中文周报。
name_zh: 中文周报
category: writing
version: 1.1.0
mode: instructions
approval: read-only-unless-writing-output
channel_capabilities: [files, markdown]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# cn-weekly-report

> **中文**：根据 git 记录、任务列表、笔记或用户口述，生成标准中文周报。
> **English**: Turn commits, tasks, notes, or user input into a Chinese weekly report.

## 适用场景 / When to use

**适用：**
- 用户需要生成本周工作总结、周报或周汇报。
- 信息源包括 git log、任务列表、笔记、文件或用户口述。

**不适用：**
- 日报（使用 cn-daily-brief）。
- 会议纪要（使用 cn-meeting-minutes）。
- 英文周报。
- 凭空生成未发生的工作内容。

## 执行步骤 / Workflow

1. 确认报告周期（默认为本周）和信息来源（git log、文件、用户口述等）。
2. 使用 **Bash**（`git log --since=... --oneline`）、**Glob**/**Grep**、**Read** 收集本周工作证据。
3. 输出 Markdown 周报，结构如下：
   - **本周完成** — 按项目或主题分组
   - **进行中** — 进度与阻塞
   - **下周计划** — 可执行项
   - **风险与需协调**（可选）
4. 默认在对话中输出；需要落盘时保存到 `reports/weekly/YYYY-WXX.md`。
5. 输出格式便于粘贴到飞书/钉钉/邮件。

## 质量契约 / Quality contract

- 数字与日期要准确；不确定处标注「待确认」。
- 不捏造未完成的工作项或虚假进度。
- 每项完成工作必须能在 git log 或用户提供的材料中找到依据。
- 语气正式、商务中文。

## 失败恢复 / Failure recovery

- git 仓库不可用时，基于用户口述或提供的文件生成周报，并标注 `[仅基于用户输入]`。
- 信息缺失时先生成「周报草稿」，单列缺失信息，不阻塞其余内容交付。
- 部分材料不可读时，单独列出并继续处理可读内容。
