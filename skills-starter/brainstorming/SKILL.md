---
name: brainstorming
description: Structured brainstorming — turn a vague idea into a concrete, validated plan before any implementation. Patterned after obra/superpowers.
description_zh: 结构化头脑风暴：在动手实现前，把模糊想法逐步追问、收敛为具体且经过验证的方案（源自 obra/superpowers 的 MIT 实践）。
name_zh: 头脑风暴
category: dev
version: 1.0.0
mode: instructions
approval: read-only-unless-writing-output
channel_capabilities: [markdown]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# brainstorming

> **中文**：在任何实现动作之前，用「一次一个问题」的追问把想法收敛成明确方案，并写成设计文档供确认。
> **English**: Before any implementation, converge a vague idea into a concrete design via one-question-at-a-time dialogue, then write it up for sign-off.

## When to use

- 用户提出新需求/新想法但细节未定（「做个 X」「加个 Y 功能」）。
- 方案存在多种可行路线，需要权衡取舍后再动手。
- 不适用于：需求已完全明确的小改动、纯问答。

## Workflow

1. **先探现状**：读相关文件/文档，弄清现有结构、约束与惯例，再开口提问（不做无依据的提问）。
2. **一次一问**：每轮只问一个问题，优先选择题（给 2-4 个选项 + 推荐项与理由），少用开放题；问题顺序：目的 → 范围边界 → 关键约束 → 取舍偏好。
3. **分段提案**：信息足够后，按模块分段提出方案（每段 ≤ 200-300 字），逐段确认，而不是一次甩出巨型设计。
4. **写设计文档**：确认后落盘到 `docs/plans/YYYY-MM-DD-<topic>-design.md`：背景、目标与非目标、方案、取舍记录、验收标准、风险。
5. **自审**：扫一遍文档——占位符（TBD）、内部矛盾、歧义、范围蔓延；发现问题就地修复后请用户确认。
6. **交接**：确认后建议进入实现（对应 plan/实现 skill）；未确认不动手写实现代码。

## Quality contract

- 每个设计决定都能回溯到用户的回答或明确的默认假设（假设需标注）。
- 不替用户决定价值取舍；技术细节可给推荐，业务偏好必须问。
- 范围控制：发现新需求时记录为「后续迭代」，不塞进当前设计。

## Failure recovery

- 用户答「都行/你定」→ 给出推荐方案 + 理由 + 反例，请用户确认推荐而非留白。
- 需求本身不成立（与现有系统冲突）→ 摆出冲突证据，提出替代目标供选择。
