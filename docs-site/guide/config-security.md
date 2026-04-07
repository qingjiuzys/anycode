---
title: Config & security
description: ~/.anycode/config.json, security fields, environment variables, and UI locale.
summary: Basic safe defaults first, then advanced policy fields and environment variables.
read_when:
  - You are tuning security, sandbox, or MCP deny rules.
  - You need locale / env var behavior for the CLI.
---

# Config & security

For users who want safe defaults first, then optional advanced controls.

After this page, you will know:

- where config is stored
- which settings are safe for normal daily use
- where to look when approvals or MCP rules block work

## Basic (recommended for most users)

Config file path:

- default: `~/.anycode/config.json`
- custom: `-c/--config <PATH>`

```bash
anycode config
```

Expected output: interactive config wizard opens and saves to config path.

Recommended defaults:

- keep `require_approval: true`
- keep `permission_mode: "default"`
- only use `--ignore-approval` for one-time debugging

One-time bypass example:

```bash
anycode run --ignore-approval --agent general-purpose "..."
```

Expected output: one task run skips approval prompts in current process only.

## Security fields (advanced)

| Field | Default | What it controls |
|---|---|---|
| `require_approval` | `true` | Ask before sensitive tools run |
| `permission_mode` | `"default"` | Shortcut mode (`default` / `auto` / `plan` / `bypass`) |
| `sandbox_mode` | `false` | Path/cwd constraints |
| `mcp_tool_deny_rules` | `[]` | Deny MCP tool calls by rule |
| `always_allow_rules` | `[]` | Always allow matching rules |
| `always_ask_rules` | `[]` | Always ask even if approval is off |
| `defer_mcp_tools` | `false` | Hide MCP tools in first model turn |

## Memory & first-turn tool choice

| Field | Default | Meaning |
|---|---|---|
| `memory.backend` | `"file"` | `file` / `hybrid` / `noop` |
| `memory.path` | `~/.anycode/memory` | Memory directory |
| `memory.auto_save` | `true` | Save memory after successful tasks |
| `zai_tool_choice_first_turn` | `false` | Prefer tool call on first turn for z.ai stack |

## System prompt overrides

Optional top-level string fields:

- `system_prompt_override`: replace default system prompt
- `system_prompt_append`: append extra content

Both support `@path` (relative to config file directory).

## MCP deny rules

- `security.mcp_tool_deny_rules`: deny by rule string
- `security.mcp_tool_deny_patterns`: deny by regex before tool exposure

## Locale (CLI UI)

Quick language setting:

```bash
export ANYCODE_LANG=zh
# or
export ANYCODE_LANG=en
```

Next step: open a new shell or re-run command in current shell, then start `anycode`.

Resolution order is `ANYCODE_LANG` -> locale env vars -> OS locale.

## Environment highlights

| Variable | Role |
|---|---|
| `ANYCODE_IGNORE_APPROVAL` | Process-level approval bypass |
| `ANYCODE_OSC8_LINKS` | Clickable OSC8 links |
| `ANYCODE_ZAI_TOOL_CHOICE_FIRST_TURN` | First-turn tool-call preference |
| `ANYCODE_ZAI_TOOL_CHOICE` | `required` / `auto` for debugging |
| `ANYCODE_MCP_COMMAND`, `ANYCODE_MCP_SERVERS` | MCP integration |
| `ANYCODE_DAEMON_TOKEN` | Daemon bearer token |

## Next

- [Models](./models) — `provider`, `model`, endpoints  
- [Troubleshooting](./troubleshooting) — common failures

