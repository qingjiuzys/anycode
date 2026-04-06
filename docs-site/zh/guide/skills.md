---
title: Agent skills（技能）
description: SKILL.md 约定、~/.anycode/skills 扫描、config.json 的 skills 段、Skill 工具与 anycode skills 子命令。
summary: anyCode 如何发现技能、写入系统提示，以及可选 run 脚本的执行方式。
read_when:
  - 需要与 OpenClaw / agentskills 风格对齐的目录与 frontmatter。
  - 要用 CLI 查看搜索根或生成技能模板。
---

# Agent skills（技能）

anyCode 采用常见的 **Agent Skills** 约定：每个技能是一个目录，内含 **`SKILL.md`**，顶部 YAML frontmatter 必填 **`name`**、**`description`**。目录下可选可执行文件 **`run`**，由 **`Skill`** 工具调用（风险级别接近 **Bash**，走审批与敏感工具策略）。

## 目录布局

- **用户级默认根目录：** `~/.anycode/skills/<skill_id>/`
- **项目内（启动时不扫描）：** `<cwd>/skills/<skill_id>/` 或 `<cwd>/.anycode/skills/<skill_id>/` — 若 catalog 中尚无该 id，在 **Skill** 工具执行时会尝试解析。
- **`skill_id`** 须与目录名、frontmatter 的 **`name`** 一致（仅 ASCII 字母数字及 `.` `_` `-`）。不一致会记录警告并跳过。

最小 **`SKILL.md`** 示例：

```markdown
---
name: my-skill
description: 给模型和 anycode skills list 用的一句话说明
---

# my-skill

给人看的正文（可选）。
```

可选 **`run`**：须为普通文件；以技能目录为 **cwd** 执行，可将 CLI 参数原样传入。

## 配置（`~/.anycode/config.json`）

**`skills`** 段字段：

| 字段 | 含义 |
|------|------|
| **`enabled`** | 为 `true` 时，启动时扫描 **`extra_dirs`** 再扫 **`~/.anycode/skills`**，生成目录并往默认系统提示栈注入 **## Available skills**（若设置了整段 **`system_prompt_override`** 则不注入）。 |
| **`extra_dirs`** | 额外搜索根（优先级低于 **`~/.anycode/skills`**；同一 id 后者覆盖前者）。 |
| **`allowlist`** | 若设置，仅这些 id 进入目录与提示。 |
| **`run_timeout_ms`** | **`run`** 子进程超时（代码侧有下限）。 |
| **`minimal_env`** | 为 `true` 时子进程仅保留少量环境变量（**PATH**、**HOME**、**USER** 等）。 |
| **`expose_on_explore_plan`** | 在 **`enabled`** 同时为 `true` 时，让 **explore** / **plan** 也注册 **Skill** 工具（默认 `false`，控制任意代码执行面）。 |

## 命令行

```bash
anycode skills list   # id、是否有 run、描述、根路径
anycode skills path   # 生效的搜索根与 skills.enabled
anycode skills init <name>   # 在 ~/.anycode/skills/<name>/ 生成 SKILL.md 与 run 模板
```

## 模型侧可见性

在启用技能且使用默认系统提示栈时，会附带 **Available skills** 列表（id + 描述）。实际执行仍通过 **Skill** 工具，例如 **`{"name": "<id>", "args": [...]}`**。

## 相关

- [配置与安全](./config-security)  
- [发现与 test-security](./cli-diagnostics)  
- [架构](./architecture)  

English: [Agent skills](/guide/skills).
