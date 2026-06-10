---
name: cn-weekly-report
description: Summarize a week's work into a Chinese weekly report for managers or teams.
description_zh: 将一周工作整理为面向团队或上级的中文周报。
---

# cn-weekly-report

> **中文**：根据 git 记录、任务列表、笔记或用户口述，生成标准中文周报。  
> **English**: Turn commits, tasks, notes, or user input into a Chinese weekly report.

## 推荐章节

1. **本周完成** — 按项目或主题分组
2. **进行中** — 进度与阻塞
3. **下周计划** — 可执行项
4. **风险与需协调**（可选）

## 规则

- 可用 **Bash**（`git log`）、**Glob**/**Grep**、**Read** 收集证据；缺信息时向用户确认。
- 数字与日期要准确；不确定处标注「待确认」。
- 输出 Markdown，便于粘贴到飞书/钉钉/邮件。
