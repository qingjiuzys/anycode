---
title: 配置与安全
description: ~/.anycode/config.json、security 字段、环境变量与界面语言。
summary: 先给普通用户推荐设置，再给进阶策略和环境变量说明。
read_when:
  - 要调审批、沙箱或 MCP 过滤规则。
  - 需要界面语言或环境变量行为说明。
---

# 配置与安全

适合先求“安全可用”，再按需打开进阶能力的用户。

完成本页后，你会知道：

- 配置文件在哪里
- 日常推荐保留哪些默认项
- 审批或 MCP 规则卡住时先看哪里

## 基础设置（大多数用户）

配置文件位置：

- 默认：`~/.anycode/config.json`
- 自定义：`-c/--config <PATH>`

```bash
anycode config
```

预期输出：进入配置向导并保存到配置文件路径。

建议默认值：

- 保持 `require_approval: true`
- 保持 `permission_mode: "default"`
- 只在临时调试时使用 `--ignore-approval`

临时跳过审批示例：

```bash
anycode run --ignore-approval --agent general-purpose "..."
```

预期输出：仅本次任务执行跳过审批提示，不会改写配置文件。

## 安全字段（进阶）

| 字段 | 默认 | 作用 |
|------|------|------|
| `require_approval` | `true` | 是否在敏感工具执行前确认 |
| `permission_mode` | `"default"` | 快捷模式（`default` / `auto` / `plan` / `bypass`） |
| `sandbox_mode` | `false` | 路径和工作目录限制 |
| `mcp_tool_deny_rules` | `[]` | 按规则拒绝 MCP 工具调用 |
| `always_allow_rules` | `[]` | 匹配后始终放行 |
| `always_ask_rules` | `[]` | 匹配后始终询问 |
| `defer_mcp_tools` | `false` | 首轮隐藏 MCP 工具 |

## 记忆与首轮工具调用

| 字段 | 默认 | 含义 |
|------|------|------|
| `memory.backend` | `"file"` | `file` / `hybrid` / `noop` |
| `memory.path` | `~/.anycode/memory` | 记忆目录 |
| `memory.auto_save` | `true` | 成功任务后自动保存 |
| `zai_tool_choice_first_turn` | `false` | z.ai 首轮更偏向 tool 调用 |

## System 提示词

可选字段：

- `system_prompt_override`：整体覆盖默认 system
- `system_prompt_append`：追加到默认 system 末尾

两者都支持 `@路径`（相对路径相对配置文件目录）。

## 模型指令文件（AGENTS.md）

anyCode 会自动发现并加载项目中的 `AGENTS.md` 文件作为模型指令。这类似于 `.cursorrules` 或其他项目级指令文件。

### 搜索位置（按顺序）

1. 工作目录：`./AGENTS.md`、`./.agents.md`、`./agents.md`、`./MODEL_INSTRUCTIONS.md`
2. `.anycode/` 子目录：`./.anycode/AGENTS.md` 等
3. 父目录（向上遍历到项目根目录，遇到 `.git`、`Cargo.toml`、`package.json` 等停止）

找到的第一个文件会被加载，并以"项目指令"部分注入到系统提示词中。

### 配置

```json
{
  "model_instructions": {
    "enabled": true,
    "filename": null,
    "max_depth": 10
  }
}
```

| 字段 | 默认 | 含义 |
|------|------|------|
| `enabled` | `true` | 启用/禁用模型指令发现 |
| `filename` | `null` | 自定义文件名（如果设置，则只搜索该文件） |
| `max_depth` | `10` | 向上遍历父目录的最大深度 |

### 示例 AGENTS.md

```markdown
# 项目规范

- 使用 TypeScript 并启用 strict 模式
- 遵循现有代码风格
- 为新功能编写测试
- 文档化公开 API
```

当项目中存在此文件时，其内容会自动包含在所有 agent 交互的系统提示词中。

## MCP 过滤

- `security.mcp_tool_deny_rules`：按规则拒绝
- `security.mcp_tool_deny_patterns`：按正则在暴露给模型前剔除

## 界面语言

快速设置：

```bash
export ANYCODE_LANG=zh
# 或
export ANYCODE_LANG=en
```

下一步：在当前 shell 重新执行 anycode 命令，或新开终端后再运行。

解析顺序是 `ANYCODE_LANG` -> 语言环境变量 -> 系统语言。

## 环境变量摘要

| 变量 | 作用 |
|------|------|
| `ANYCODE_IGNORE_APPROVAL` | 进程级跳过审批（见 `--help`） |
| `ANYCODE_OSC8_LINKS` | 终端 OSC 8 可点击链接 |
| `ANYCODE_ZAI_TOOL_CHOICE_FIRST_TURN` | z.ai 首轮强制 tool_calls |
| `ANYCODE_ZAI_TOOL_CHOICE` | 每轮 `required` / `auto`（调试用） |
| `ANYCODE_MCP_COMMAND` / `ANYCODE_MCP_SERVERS` | MCP（需 `tools-mcp`） |
| `ANYCODE_DAEMON_TOKEN` | 守护进程 POST 鉴权 |

## 下一步

- [模型与端点](./models)  
- [排错](./troubleshooting)  
