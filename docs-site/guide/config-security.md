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
| `permission_mode` | `"default"` | Shortcut mode (`default` / `auto` / `plan` / `accept_edits` / `bypass`) |
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

## Model instructions file (AGENTS.md)

anyCode automatically discovers and loads model instructions from `AGENTS.md` files in your project. This is similar to `.cursorrules` or other project-specific instruction files.

### Search locations (in order)

1. Working directory: `./AGENTS.md`, `./.agents.md`, `./agents.md`, `./MODEL_INSTRUCTIONS.md`
2. `.anycode/` subdirectory: `./.anycode/AGENTS.md`, etc.
3. Parent directories (up to project root, stops at `.git`, `Cargo.toml`, `package.json`, etc.)

The first file found is loaded and injected as a **Project Instructions** section in the system prompt.

### Explicit file (environment variable)

To load a specific file without using discovery, set:

```bash
export ANYCODE_MODEL_INSTRUCTIONS_FILE=/absolute/or/relative/path/to/instructions.md
```

Relative paths are resolved against the **process working directory**. This is **only** an environment variable: there is no `model_instructions_file` (or similar) field in `config.json`. Use the `model_instructions` JSON object below to tune **discovery** (enable/disable, custom filename, walk depth).

### When both explicit and discovery apply

If `ANYCODE_MODEL_INSTRUCTIONS_FILE` is set **and** discovery finds a file, the runtime may inject **both**, in this order:

1. **Model Instructions** — content from the explicit path.
2. **Project Instructions** — content from the first discovery match.

### Configuration

```json
{
  "model_instructions": {
    "enabled": true,
    "filename": null,
    "max_depth": 10
  }
}
```

| Field | Default | Meaning |
|---|---|---|
| `enabled` | `true` | Enable/disable model instructions discovery |
| `filename` | `null` | Custom filename (if set, only searches for this file) |
| `max_depth` | `10` | Max parent directories to traverse |

### Example AGENTS.md

```markdown
# Project Guidelines

- Use TypeScript with strict mode enabled
- Follow the existing code style
- Write tests for new features
- Document public APIs
```

When this file exists in your project, the content will be automatically included in the system prompt for all agent interactions.

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

## Approval matrix (quick reference)

| Surface | Policy entry | Notes |
|---|---|---|
| TUI / `run` / `repl` | `security.require_approval` + `permission_mode` | Interactive prompts when stdin is a TTY; **`--ignore-approval`** applies to **this process only**. |
| Channel bridges (WeChat / Telegram / Discord) | Same config file | Runtime uses **`WorkspaceAssistantAgent`** for **`RuntimeMode::Channel`** — read/search/workflow-first tools; coding tools are not the default set. |
| Goal loops | Same **`SecurityLayer`** as the parent runtime | Use **`GoalSpec.max_attempts_cap`** to bound retries even when **`allow_infinite_retries`** is true. |
| Feature flags | `anycode enable approval-v2` | Maps to **`FeatureFlag::ApprovalV2`** (experimental tooling). |

## Next

- [Models](./models) — `provider`, `model`, endpoints  
- [Troubleshooting](./troubleshooting) — common failures

