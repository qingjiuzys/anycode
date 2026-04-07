---
title: run / REPL / TUI
description: anycode run、repl、全屏 TUI、任务日志与终端链接行为。
summary: 单次任务、行式 REPL、默认 TUI、stdout/stderr 分工与 OSC 8。
read_when:
  - 在 TUI 与 repl 或脚本 run 之间选择。
  - 需要日志路径或工具调用验收 grep 提示。
---

# run / REPL / TUI

## 用户工作区（`~/.anycode/workspace`）

**用户级默认工作区根**，与 **`~/.anycode/wechat`** 并列。

| 路径 | 作用 |
|------|------|
| `README.md` | 首次创建时的简短说明 |
| `projects/index.json` | 从各目录运行 TUI / `repl` / `run` 时登记有效工作目录（`run -C` / `repl -C` 优先，否则当前目录），按 `last_seen` 排序，约 200 条上限 |

任务 cwd 仍为当前目录或 **`-C`**；全局 Memory 仍在 **`config.json` 的 `memory.path`**。

## `run`

```bash
./target/release/anycode run --agent general-purpose "请只回复：OK"
./target/release/anycode run --agent plan "为这个仓库设计一份技术路线图"
./target/release/anycode run -C /path/to/repo --agent general-purpose "分析此目录"
```

落盘：**`~/.anycode/tasks/<task_id>/output.log`**。

**stdout / stderr：** 任务日志路径、执行中提示、完成/失败摘要 → **stderr**；增量 tail 与最终 **Output** 正文 → **stdout**。若有 **FileWrite**，stderr 多一节 **Written**。便于 `2>/dev/null` 或脚本分流。

### 验收（工具是否真正执行）

日志中可搜索：

- `[tool_call_start]`、`[tool_call_input]`、`[tool_call_end]`

### z.ai 与 Anthropic

z.ai 走 OpenAI 兼容 **`tool_calls`**；部分模型首轮可能只回文本。可设：

| 环境变量 | 作用 |
|----------|------|
| `ANYCODE_ZAI_TOOL_CHOICE_FIRST_TURN=1` | 首轮（仅 system+user）且带 tools 时 `tool_choice: required` |
| `ANYCODE_ZAI_TOOL_CHOICE=required` | 每轮 required（通常仅调试） |
| `ANYCODE_ZAI_TOOL_CHOICE=auto` | 显式恢复默认 |

或与 `config.json` 中 **`zai_tool_choice_first_turn`** 等价；**环境变量优先**。

示例：

```bash
ANYCODE_ZAI_TOOL_CHOICE_FIRST_TURN=1 ./target/release/anycode run --agent general-purpose "请用 Bash 执行：echo OK"
```

## `repl`

```bash
./target/release/anycode repl
./target/release/anycode repl --agent explore -C /path/to/repo
./target/release/anycode repl --model glm-5
```

- **`--model`**：仅本进程，**不写回** `config.json`。  
- 欢迎页在 stdout；默认不把 **`tracing`** INFO 打到 stderr — 排障用 **`anycode --debug repl`** 或 **`RUST_LOG`**。  
- 提示符 **`anycode>`**；每行一次与 `run` 相同的 **`execute_task`**。  
- 审批：与 `run` 一致；**`-I/--ignore-approval`** 时欢迎框提示已跳过。

## 全屏 TUI（无子命令）

```bash
./target/release/anycode
./target/release/anycode --model glm-5
```

**仅 `--model` 长选项**（无 `-m`）。

底部输入：**`/help`**、**`/agents`**、**`/tools`**、**`/exit`**；普通回车为一轮对话，共享 **messages** 历史。

**注意：** 无子命令 TUI 使用**当前目录**与 **`general-purpose`**；需要其它 agent 或目录请用 **`repl`** / **`run`**。

### TUI 内 Markdown 与链接

| 环境变量 | 行为 |
|----------|------|
| 未设置 | 链接下划线 + 灰色完整 URL |
| `ANYCODE_OSC8_LINKS=1` | OSC 8，iTerm2 / Kitty / WezTerm / Windows Terminal 可 ⌘/Ctrl+点击 |

## 相关

- [配置与安全](./config-security) — 记忆、`require_approval` / `permission_mode`  
- [微信与 setup](./wechat)  

English: [Run, REPL & TUI](/guide/cli-sessions).
