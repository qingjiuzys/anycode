---
title: Discovery & test-security
description: list-agents, list-tools, and test-security for local policy debugging.
summary: Introspection commands and SecurityLayer check_tool_call helper.
read_when:
  - You need to list agents or tools from the CLI.
  - You debug why a tool call was denied.
---

# Discovery & test-security

## List agents and tools

```bash
anycode list-agents
anycode list-tools
```

When **Agent** or **Task** appears in the tool list, those invoke nested sub-agents. JSON inputs and behavior aligned with Claude Code’s **`Agent`** tool (**`cwd`**, **`model`**, **`isolation`**, **`run_in_background`**, etc.) are documented under **P5** in the [Roadmap](./roadmap).

## `test-security`

Runs **`SecurityLayer::check_tool_call`** for a given tool name and JSON input:

```bash
anycode test-security --tool Bash --input '{"command":"ls"}'
```

Exact subcommand spelling is shown in **`anycode --help`**.

## `LSP` tool

With **`--features tools-lsp`**, the **`LSP`** tool forwards JSON-RPC over a stdio subprocess. Configure **`lsp`** in `config.json` (see [Config & security](./config-security) — **LSP**) or set **`ANYCODE_LSP_COMMAND`**.

## Related

- [Roadmap](./roadmap) — **P5**: **Agent** / **Task** vs Claude field parity  
- [Agent skills](./skills) — **`anycode skills`**, **`SKILL.md`** discovery  
- [Config & security](./config-security) — deny rules and **`permission_mode`**  
- [Architecture](./architecture) — **SecurityLayer** wiring  
