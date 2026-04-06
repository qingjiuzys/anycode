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

## `test-security`

Runs **`SecurityLayer::check_tool_call`** for a given tool name and JSON input:

```bash
anycode test-security --tool Bash --input '{"command":"ls"}'
```

Exact subcommand spelling is shown in **`anycode --help`**.

## Related

- [Agent skills](./skills) — **`anycode skills`**, **`SKILL.md`** discovery  
- [Config & security](./config-security) — deny rules and **`permission_mode`**  
- [Architecture](./architecture) — **SecurityLayer** wiring  
