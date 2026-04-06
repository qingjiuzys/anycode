---

## title: Config & security

description: ~/.anycode/config.json, security fields, environment variables, and UI locale.
summary: Where settings live, how approval and permission_mode interact, and ANYCODE_* highlights.
read_when:

- You are tuning security, sandbox, or MCP deny rules.
- You need locale / env var behavior for the CLI.

# Config & security

Default config path: `**~/.anycode/config.json`**. If you pass `**-c/--config <PATH>`** and the file is missing, the CLI errors. Subcommands that read/write config use this path.

```bash
anycode config
```

The wizard preserves existing `**routing**` and `**security**` sections. After save on a TTY you may be offered the same WeChat bind flow as `anycode wechat` (see [WeChat & onboard](./wechat)).

## Security & approval

In `config.json`, the `**security**` object commonly includes:


| Field                 | Default     | Meaning                                                                                                                                                                                                                                 |
| --------------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `require_approval`    | `true`      | If `true`, sensitive tools prompt y/n in TUI or stdin for `run` / REPL. If `false`, those tools skip interactive approval (deny rules still apply), **unless** `always_ask_rules` still requires a prompt (Claude-style **alwaysAsk**). |
| `permission_mode`     | `"default"` | Layer *before* policy/approval: `default` (no shortcut), `auto` (read-only tools auto-approved in `SecurityLayer`), `plan` (reserved; same as `default` today), `bypass` (**skips** policy checks and deny — local debug only).         |
| `sandbox_mode`        | `false`     | Path / cwd constraints (see root README).                                                                                                                                                                                               |
| `mcp_tool_deny_rules` | `[]`        | Blanket **alwaysDeny**-style rules for tools (including `mcp__Server` / `mcp__Server__`*). Also used with allow/ask lists from the same rule string format.                                                                             |
| `always_allow_rules`  | `[]`        | **alwaysAllow** (blanket or `Tool(content)`); can override content-level denies at execution time.                                                                                                                                      |
| `always_ask_rules`    | `[]`        | **alwaysAsk**; matching tool calls need interactive approval even when `require_approval` is `false` for sensitive tools.                                                                                                               |
| `defer_mcp_tools`     | `false`     | Hide all `mcp__`* from the first LLM tool list until registered (Claude defer MCP).                                                                                                                                                     |


**Claude Code–style mapping:** `mcp_tool_deny_rules` ≈ alwaysDeny, `always_allow_rules` ≈ alwaysAllow, `always_ask_rules` ≈ alwaysAsk (see roadmap / tools docs for rule string syntax).

`**-I` / `--ignore-approval` / `ANYCODE_IGNORE_APPROVAL`:** skips **all** interactive tool approval for that process, including **alwaysAsk** (`always_ask_rules`), without writing the config file.

**To get “auto approve” without disabling deny rules:** set `"require_approval": false`, leave `**always_ask_rules` empty**, and **do not** rely on `permission_mode: "bypass"`.

**This process only**, without editing the file:

```bash
anycode --ignore
anycode run --ignore-approval --agent general-purpose "…"
```

## Memory & first-turn tool choice


| Field                        | Default                             | Meaning                                                                                                                     |
| ---------------------------- | ----------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `memory.backend`             | `"file"`                            | `file` / `hybrid` / `noop` / `none` / `off` — see [CLI sessions](./cli-sessions).                                           |
| `memory.path`                | (default under `~/.anycode/memory`) | Root for memory files; relative paths are under `$HOME`.                                                                    |
| `memory.auto_save`           | `true`                              | Auto-save project memory after successful tasks when backend is active.                                                     |
| `zai_tool_choice_first_turn` | `false`                             | First turn `tool_choice: required` on OpenAI-compatible stack; `**ANYCODE_ZAI_TOOL_CHOICE_FIRST_TURN` overrides** when set. |


## System prompt overrides

Optional top-level string fields (or `@path` relative to the config file directory):

- `**system_prompt_override`** — replaces the entire default system message when non-empty.
- `**system_prompt_append`** — appended after the composed system message.

WeChat bridge `config.env` `**systemPrompt`** is treated like `**system_prompt_append`**.

## MCP deny rules

- `**security.mcp_tool_deny_rules`** — blanket rules.  
- `**security.mcp_tool_deny_patterns**` — regex list to strip tools before the model sees them.

Details: [Roadmap](./roadmap) (tools / MCP sections) and root README.

## Locale (CLI UI)

Resolved in order: `**ANYCODE_LANG**` / `**LANGUAGE**`, then `**LC_ALL**` / `**LC_MESSAGES**` / `**LANG**`, then OS locale. Examples:

```bash
export ANYCODE_LANG=zh   # or en
```

Model-facing system prompts and tool descriptions default to **English** for stability.

## Environment highlights


| Variable                                     | Role                                                                      |
| -------------------------------------------- | ------------------------------------------------------------------------- |
| `ANYCODE_IGNORE_APPROVAL`                    | Process-level approval skip (see CLI help).                               |
| `ANYCODE_OSC8_LINKS`                         | OSC 8 hyperlinks in terminal output (see [CLI sessions](./cli-sessions)). |
| `ANYCODE_ZAI_TOOL_CHOICE_FIRST_TURN`         | First-turn tool calls on z.ai / OpenAI-compatible.                        |
| `ANYCODE_ZAI_TOOL_CHOICE`                    | `required` / `auto` per-turn (debug).                                     |
| `ANYCODE_MCP_COMMAND`, `ANYCODE_MCP_SERVERS` | MCP when built with `tools-mcp`.                                          |
| `ANYCODE_DAEMON_TOKEN`                       | Bearer for daemon `POST /v1/tasks`.                                       |


Full tables: root README and [CLI overview](./cli).

## Next

- [Models](./models) — `provider`, `model`, endpoints  
- [Troubleshooting](./troubleshooting) — common failures

