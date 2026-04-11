---
title: Run, REPL & TUI
description: anycode run, repl, fullscreen TUI, task logs, and terminal link behavior.
summary: One-shot tasks, line REPL, default TUI, stdout/stderr split, and OSC 8 links.
read_when:
  - You choose between TUI and repl or script run.
  - You need log paths or tool-call grep hints.
---

# Run, REPL & TUI

## Default entry: what runs when you type `anycode`

| How you start | Interactive TTY? | What you get |
|---------------|------------------|----------------|
| **`anycode`** (no subcommand) | yes | **Inline stream REPL** — ratatui viewport + dock, shared **messages** engine with TUI; use **`anycode --resume <uuid>`** to continue a saved session. |
| **`anycode`** | no | Line-at-a-time **stdio** mode (no ratatui). |
| **`anycode repl`** | yes | Same **stream REPL** as above; use this when you want **`-C`**, **`--agent`**, or **`--resume`** explicitly in the command line. |
| **`anycode tui`** | — | **Fullscreen TUI** (see below). |

Session snapshots live under **`~/.anycode/tui-sessions/`** (same format for stream REPL and TUI).

**Scrollback after exiting stream REPL:** by default the CLI prints the **full** inline transcript again so you can search it in the shell. To reduce duplication with the viewport:

- **`ANYCODE_STREAM_EXIT_SCROLLBACK_DUMP=0`** (or `false` / `no` / `off`) — do not print.
- **`ANYCODE_STREAM_EXIT_SCROLLBACK_DUMP=anchor`** — print only from the **last natural-language turn** (byte offset captured when that turn started; same anchor the streamer uses when rebuilding the plain-text buffer).
- **`full`**, **`1`**, **`true`**, or unset — full transcript (default).

**Read-only usage:** **`/context`** and **`/cost`** in the host slash menu show message counts, configured context window, and last-turn token aggregates where available. **`/cost`** does **not** estimate dollars — provider billing is authoritative.

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

On an **interactive TTY**, **`anycode repl`** uses the same **Inline stream REPL** as **`anycode`** (ratatui viewport + dock, multi-turn **`execute_turn_from_messages`**). Without a TTY (pipes / scripts), it falls back to **line-at-a-time stdio** (closer to “classic” line REPL).

```bash
anycode repl
anycode repl --agent explore -C /path/to/repo
anycode repl --model glm-5
anycode repl --resume <uuid>
```

- **`--model`** applies **this process only**; does not write **`config.json`**.  
- Welcome banner on stdout; **`tracing`** INFO is suppressed from stderr by default — use **`anycode --debug repl`** or **`RUST_LOG`** for diagnostics.  
- Approval: same as **`run`** when **`require_approval`** is true and stdin is a TTY; **`-I/--ignore-approval`** noted in the welcome box.

## Fullscreen TUI (`anycode tui`)

```bash
anycode tui
anycode tui --model glm-5
```

**`--model`** is **long-only** here (no **`-m`** shorthand). You can also pass **`--resume <uuid>`**.

Bottom input:

- **`/help`**, **`/agents`**, **`/tools`**, **`/context`**, **`/cost`**, **`/exit`**, and other host slash commands
- Normal lines start an agentic turn with shared **messages** history.

### Slash commands: host vs prompt text

- **Host-executed**: In TUI / REPL, a **first line** starting with **`/`** is handled by the CLI (completion, `/compact`, `/mode`, etc.) before the model sees the message.
- **Prompt templates**: If you put **`/foo`** inside **`system_prompt_override`**, **`system_prompt_append`**, or skill text, it is **not** automatically executed—it is plain text unless you build a custom pipeline. The default system prompt reminds the model of this distinction.

**Note:** **`anycode`** / **`anycode repl`** on a TTY default to **stream REPL** with **current cwd** and agent from **`runtime.default_mode`** (often **`general-purpose`**). Use **`repl` / `run`** for **`-C`** and **`--agent`**. Use **`anycode tui`** when you want the fullscreen layout.

**Terminal buffer:** The default fullscreen TUI enters the **DEC alternate screen** (isolated viewport, OpenClaw-style). For **main-buffer scrollback** (closer to Claude Code without fullscreen / `CLAUDE_CODE_NO_FLICKER`): **`export ANYCODE_TUI_ALT_SCREEN=0`** before **`anycode tui`**, or run **`ANYCODE_TUI_ALT_SCREEN=0 anycode tui`** on one line — a standalone `VAR=0` line does **not** export to the child process. Alternatively set **`"tui": { "alternateScreen": false }`** in `config.json`.

### Markdown and links in TUI

Assistant text is rendered as **Markdown** (CommonMark / GFM subset).

| Env | Behavior |
|-----|----------|
| (default) | Underlined link text + gray full URL for copy. |
| **`ANYCODE_OSC8_LINKS=1`** | OSC 8 hyperlinks — **⌘/Ctrl+click** in iTerm2, Kitty, WezTerm, Windows Terminal. Unsupported terminals may show escape noise; unset if so. |

```bash
ANYCODE_OSC8_LINKS=1 anycode tui
```

## Related

- [Config & security](./config-security) — **`memory.*`**, approval matrix  
- [WeChat & setup](./wechat) — workspace + bridge  
