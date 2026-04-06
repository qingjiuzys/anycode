---
title: Run, REPL & TUI
description: anycode run, repl, fullscreen TUI, task logs, and terminal link behavior.
summary: One-shot tasks, line REPL, default TUI, stdout/stderr split, and OSC 8 links.
read_when:
  - You choose between TUI and repl or script run.
  - You need log paths or tool-call grep hints.
---

# Run, REPL & TUI

## User workspace (`~/.anycode/workspace`)

Default **user-level project root** (next to **`~/.anycode/wechat`**). Recent working directories are recorded when you use TUI, **`repl`**, or **`run`** (see **`projects/index.json`** in the Chinese guide mirror for format). Task **cwd** remains the current directory or **`-C`**; the workspace is for defaults and WeChat **`workingDirectory`**.

## `run`

```bash
anycode run --agent general-purpose "Reply with OK only"
anycode run --agent plan "Design a technical roadmap"
anycode run -C /path/to/repo --agent general-purpose "Analyze this tree"
```

Logs are written under **`~/.anycode/tasks/<task_id>/output.log`**.

**Streams:** **`run`** (and single-turn **`repl`**) writes task log path, progress hints, and completion summary to **stderr**; incremental tail and final **Output** body to **stdout**. **FileWrite** paths appear on stderr. This helps **`2>/dev/null`** or splitting streams in scripts.

### Verifying tool execution

If the model issues tool calls, the log may contain markers such as:

- **`[tool_call_start]`**, **`[tool_call_input]`**, **`[tool_call_end]`**

Use **`grep`** on **`output.log`** for quick acceptance checks.

### z.ai / OpenAI-compatible vs Anthropic

Anthropic typically returns **`tool_use`**. z.ai uses OpenAI-shaped **`tool_calls`**. Under **`tool_choice: auto`**, some models may answer with text only. Mitigations:

| Env | Effect |
|-----|--------|
| **`ANYCODE_ZAI_TOOL_CHOICE_FIRST_TURN=1`** | First turn only (system + user) with tools sends **`tool_choice: required`**. |
| **`ANYCODE_ZAI_TOOL_CHOICE=required`** | Every tool turn (**debug**; can force extra calls). |
| **`ANYCODE_ZAI_TOOL_CHOICE=auto`** | Explicit default. |

Or set **`zai_tool_choice_first_turn`** in **`config.json`** (env wins when set).

## `repl`

Line-mode session with **native terminal scroll**:

```bash
anycode repl
anycode repl --agent explore -C /path/to/repo
anycode repl --model glm-5
```

- **`--model`** applies **this process only**; does not write **`config.json`**.  
- Welcome banner on stdout; **`tracing`** INFO is suppressed from stderr by default — use **`anycode --debug repl`** or **`RUST_LOG`** for diagnostics.  
- Prompt **`anycode>`**; each line runs the same **`execute_task`** path as **`run`**.  
- Approval: same as **`run`** when **`require_approval`** is true and stdin is a TTY; **`-I/--ignore-approval`** noted in the welcome box.

## Fullscreen TUI (no subcommand)

```bash
anycode
anycode --model glm-5
```

**`--model`** is **long-only** here (no **`-m`** shorthand).

Bottom input:

- **`/help`**, **`/agents`**, **`/tools`**, **`/exit`**
- Normal lines start an agentic turn with shared **messages** history.

**Note:** Default TUI uses **current cwd** and agent **`general-purpose`**; **`repl`** / **`run`** accept **`-C`** and **`--agent`**.

### Markdown and links in TUI

Assistant text is rendered as **Markdown** (CommonMark / GFM subset).

| Env | Behavior |
|-----|----------|
| (default) | Underlined link text + gray full URL for copy. |
| **`ANYCODE_OSC8_LINKS=1`** | OSC 8 hyperlinks — **⌘/Ctrl+click** in iTerm2, Kitty, WezTerm, Windows Terminal. Unsupported terminals may show escape noise; unset if so. |

```bash
ANYCODE_OSC8_LINKS=1 anycode
```

## Related

- [Config & security](./config-security) — **`memory.*`**, approval matrix  
- [WeChat & onboard](./wechat) — workspace + bridge  
