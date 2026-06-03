---
title: Agent 与 Skills
description: 内置 agent、声明式 profile、Skills 治理与数据库持久化。
---

# Agent 与 Skills

详见英文版 [Agents & Skills](/guide/agents)（内容同步）。要点：

- 内置 5 种 agent + `summary`（仅 routing）
- 预置角色 profile：`builder`、`reviewer` 等
- `config.json` → `agents.profiles` 自定义
- Skills 生效集 = 全局 allowlist ∩ agent allowlist ∩ `project_skills`
- Dashboard **设置 → Agent** 可 CRUD；数据写入 `config.json` 与 `agent_profiles` 表

## 相关

- [路由](/zh/guide/routing)
- [Models](/guide/models)
