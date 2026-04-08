---
title: Memory notes
description: anyCode memory stores vs OpenClaw-style memory extensions.
summary: Current backends, scopes, and a short parity backlog.
read_when:
  - You compare memory behavior with OpenClaw or Claude Code.
---

# Memory notes

## Current anyCode behavior

- **Backends**: `memory.backend` supports `file`, `hybrid`, or `noop` (see [Config & security](./config-security)).
- **Scope**: Project vs user memories flow through `anycode_memory` with keyword retrieval today.
- **Autosave**: Controlled by `memory.auto_save` and successful task completion hooks in the agent runtime.

## OpenClaw parity (research backlog)

OpenClaw ships memory as an **extension** with its own retention and recall policies. A practical parity checklist:

1. **Write triggers**: when notes are created (task success only vs tool-driven vs explicit commands).
2. **Retrieval**: keyword vs embedding / hybrid; per-project isolation guarantees.
3. **Compaction interaction**: how memories survive `/compact` and automatic session compression.

Track improvements against this list in issue tracker milestones rather than duplicating OpenClaw internals inside the CLI binary.

## Related

- [Architecture](./architecture)  
- [Config & security](./config-security)  
