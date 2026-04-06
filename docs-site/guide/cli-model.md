---
title: Model commands
description: anycode model list, status, set, and interactive model editor.
summary: Static z.ai catalog vs free-form ids for other providers; writes config when using set.
read_when:
  - You switch default models or routing defaults from the CLI.
---

# Model commands

```bash
anycode model list --plain
anycode model status
anycode model set <id>
```

**`model list`** is primarily a **z.ai** static catalog. For **Anthropic**, set **`provider`** and **`model`** directly in **`config.json`**.

Interactive **`anycode model`** (no subcommand) edits global defaults and **`routing.agents`** as implemented in your version.

All of these respect **`-c/--config`**.

## Related

- [Models](./models) — providers and endpoints  
- [Routing](./routing) — per-agent overrides  
