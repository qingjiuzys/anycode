---
title: 配置与安全
description: ~/.anycode/config.json、security 字段、环境变量与界面语言。
summary: 配置路径、审批与 permission_mode、记忆与 ANYCODE_* 要点。
read_when:
  - 要调审批、沙箱或 MCP 过滤规则。
  - 需要界面语言或环境变量行为说明。
---

# 配置与安全

默认配置文件：**`~/.anycode/config.json`**。若显式传入 **`-c/--config <PATH>`** 且文件不存在，CLI 会报错。会读写的子命令均使用该路径。

```bash
anycode config
```

向导会保留已有 **`routing`** 与 **`security`** 段。保存成功后，在 TTY 下可能询问是否绑定微信（与 `anycode channel wechat` 一致，见 [微信与 setup](./wechat)）。

## 安全与审批

`security` 常用字段：

| 字段 | 默认 | 含义 |
|------|------|------|
| `require_approval` | `true` | `true`：TUI / 连着终端的 `run` 等对敏感工具弹 y/n；`false`：上述工具不弹交互（**deny 仍生效**），但若配置了 **`always_ask_rules`**（Claude **alwaysAsk**），命中规则时仍会要确认。 |
| `permission_mode` | `"default"` | 在策略与审批之上的一层：`default`；`auto`（只读类工具在 `SecurityLayer` 内直接放行）；`plan`（预留，现同 `default`）；`bypass`（**整段跳过**策略与 deny，仅建议本机调试）。 |
| `sandbox_mode` | `false` | 路径/工作目录约束等（见根目录 README）。 |
| `mcp_tool_deny_rules` | `[]` | **alwaysDeny** 风格 blanket 规则（含 `mcp__Server` / `mcp__Server__*`）；与同格式的 allow/ask 列表一起编译。 |
| `always_allow_rules` | `[]` | **alwaysAllow**（blanket 或 `Tool(content)`）；可在执行前覆盖 content 级 deny。 |
| `always_ask_rules` | `[]` | **alwaysAsk**；命中时工具调用需交互确认，即使 `require_approval` 为 `false`。 |
| `defer_mcp_tools` | `false` | 首轮对 LLM 隐藏全部 `mcp__*`，直至登记（对齐 Claude defer MCP）。 |

**与 Claude Code 权限的对应关系：** `mcp_tool_deny_rules` ≈ alwaysDeny，`always_allow_rules` ≈ alwaysAllow，`always_ask_rules` ≈ alwaysAsk（规则串语法见 roadmap / 工具文档）。

**`-I` / `--ignore-approval` / `ANYCODE_IGNORE_APPROVAL`：** 本进程跳过**全部**交互式工具审批（含 **alwaysAsk**），**不**改写配置文件。

**想要「自动通过」又不想关掉 deny**：设 `"require_approval": false`，**且** `always_ask_rules` **留空**，**不要**用 `"permission_mode": "bypass"`。

**仅本次进程、不改文件**：

```bash
anycode --ignore
anycode run --ignore-approval --agent general-purpose "…"
```

## 记忆与首轮工具调用

| 字段 | 默认 | 含义 |
|------|------|------|
| `memory.backend` | `"file"` | `file` / `hybrid` / `noop` / `none` / `off`，详见 [run / REPL / TUI](./cli-sessions)。 |
| `memory.path` | （省略则 `~/.anycode/memory`） | 记忆根目录；**相对路径相对于 `$HOME`**。 |
| `memory.auto_save` | `true` | 成功任务后自动写入 `Project` 记忆（后端非 noop 时）。 |
| `zai_tool_choice_first_turn` | `false` | OpenAI 兼容栈首轮 `tool_choice: required`；**环境变量优先**。 |

## System 提示词

顶层可选（字符串或 `@路径`，**相对路径相对配置文件目录**）：

- **`system_prompt_override`** — 非空则整条 system 仅此内容。  
- **`system_prompt_append`** — 接在合成 system 末尾。  

微信 `config.env` 的 **`systemPrompt`** 等价于 **`system_prompt_append`**。

## MCP 过滤

- **`security.mcp_tool_deny_rules`**  
- **`security.mcp_tool_deny_patterns`** — 正则列表，在交给模型前剔除工具名。  

详见 [路线图](./roadmap) 与 README。

## 界面语言

解析顺序：**`ANYCODE_LANG`** / **`LANGUAGE`** → **`LC_ALL`** / **`LC_MESSAGES`** / **`LANG`** → 系统区域。示例：

```bash
export ANYCODE_LANG=zh   # 或 en
```

面向模型的 system / 工具描述默认仍以 **英文** 为主。

## 环境变量摘要

| 变量 | 作用 |
|------|------|
| `ANYCODE_IGNORE_APPROVAL` | 进程级跳过审批（见 `--help`） |
| `ANYCODE_OSC8_LINKS` | 终端 OSC 8 可点击链接 |
| `ANYCODE_ZAI_TOOL_CHOICE_FIRST_TURN` | z.ai 首轮强制 tool_calls |
| `ANYCODE_ZAI_TOOL_CHOICE` | 每轮 `required` / `auto`（调试用） |
| `ANYCODE_MCP_COMMAND` / `ANYCODE_MCP_SERVERS` | MCP（需 `tools-mcp`） |
| `ANYCODE_DAEMON_TOKEN` | 守护进程 POST 鉴权 |

完整表：根 README、[CLI 总览](./cli)。

## 下一步

- [模型与端点](./models)  
- [排错](./troubleshooting)  
