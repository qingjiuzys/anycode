---
title: run / REPL / TUI
description: anycode run、repl、全屏 TUI、任务日志与终端链接行为。
summary: 单次任务、行式 REPL、默认 TUI、stdout/stderr 分工与 OSC 8。
read_when:
  - 在 TUI 与 repl 或脚本 run 之间选择。
  - 需要日志路径或工具调用验收 grep 提示。
---

# run / REPL / TUI

## 默认入口：直接打 `anycode` 时是什么界面

| 启动方式 | 交互 TTY？ | 实际界面 |
|----------|------------|----------|
| **`anycode`**（无子命令） | 是 | **Inline 流式 REPL**（ratatui 视口 + 底栏），与全屏 TUI **共用 messages 引擎**；可用 **`anycode --resume <uuid>`** 续聊已存会话。 |
| **`anycode`** | 否 | **stdio 逐行**模式（无 ratatui）。 |
| **`anycode repl`** | 是 | 与上一种 **相同的流式 REPL**；需要显式 **`-C` / `--agent` / `--resume`** 时用此子命令。 |
| **`anycode tui`** | — | **全屏 TUI**（见下文）。 |

会话快照目录：**`~/.anycode/tui-sessions/`**（流式与 TUI 同一套 JSON）。

**退出流式 REPL 后的滚动历史：** 默认会把 **完整** transcript 再打一份到 shell，便于检索。若与视口重复可改用：

- **`ANYCODE_STREAM_EXIT_SCROLLBACK_DUMP=0`**（或 `false` / `no` / `off`）— 不打印。
- **`ANYCODE_STREAM_EXIT_SCROLLBACK_DUMP=anchor`** — 只打印 **上一轮自然语言会话** 起的内容（与流式重建 plain 时用的字节锚点一致）。
- **`full`**、**`1`**、**`true`** 或未设置 — 全文（默认）。

**只读用量：** 宿主斜杠 **`/context`**、**`/cost`** 可查看消息条数、配置的上下文窗口、上一轮 token 聚合（若有）。**`/cost` 不提供货币金额**，计费以各提供商账单为准。

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

在**交互 TTY** 上，**`anycode repl`** 与无子命令的 **`anycode`** 一样走 **Inline 流式 REPL**（ratatui 视口 + 底栏，多轮 **`execute_turn_from_messages`**）。非 TTY（管道/脚本）时回退为 **stdio 逐行**。

```bash
./target/release/anycode repl
./target/release/anycode repl --agent explore -C /path/to/repo
./target/release/anycode repl --model glm-5
./target/release/anycode repl --resume <uuid>
```

- **`--model`**：仅本进程，**不写回** `config.json`。  
- 欢迎页在 stdout；默认不把 **`tracing`** INFO 打到 stderr — 排障用 **`anycode --debug repl`** 或 **`RUST_LOG`**。  
- 审批：与 `run` 一致；**`-I/--ignore-approval`** 时欢迎框提示已跳过。

## 全屏 TUI（`anycode tui`）

```bash
./target/release/anycode tui
./target/release/anycode tui --model glm-5
```

**仅 `--model` 长选项**（无 `-m`）。可带 **`--resume <uuid>`**。

底部输入：**`/help`**、**`/agents`**、**`/tools`**、**`/context`**、**`/cost`**、**`/exit`** 等宿主斜杠；普通回车为一轮对话，共享 **messages** 历史。

### 斜杠命令：宿主 vs 提示词正文

- **宿主执行**：TUI / REPL 下输入**首行**以 **`/`** 开头时由 CLI 处理（补全、`/compact`、`/mode` 等）。
- **提示词模板**：写在 **`system_prompt_override` / `system_prompt_append`** 或 skill 中的 **`/foo`** 仅为文本，不会自动执行；默认 system 中会说明该边界。

**注意：** TTY 下 **`anycode`** / **`anycode repl`** 默认为**流式 REPL**，目录为**当前 cwd**，agent 来自 **`runtime.default_mode`**（常为 **`general-purpose`**）。指定目录/agent 用 **`repl` / `run`**；要**全屏布局**用 **`anycode tui`**。

**终端画布：** 全屏 TUI 默认 **DEC 备用屏**。需要 **主缓冲 + 终端滚动**时：先 **`export ANYCODE_TUI_ALT_SCREEN=0`** 再运行 **`anycode tui`**，或 **`ANYCODE_TUI_ALT_SCREEN=0 anycode tui` 同一行**；单独一行 `VAR=0` **不会**传给子进程。也可在 **`config.json`** 设 **`"tui": { "alternateScreen": false }`**。

### TUI 内 Markdown 与链接

| 环境变量 | 行为 |
|----------|------|
| 未设置 | 链接下划线 + 灰色完整 URL |
| `ANYCODE_OSC8_LINKS=1` | OSC 8（全屏 TUI），iTerm2 / Kitty / WezTerm / Windows Terminal 可 ⌘/Ctrl+点击 |

## 相关

- [配置与安全](./config-security) — 记忆、`require_approval` / `permission_mode`  
- [微信与 setup](./wechat)  

English: [Run, REPL & TUI](/guide/cli-sessions).
