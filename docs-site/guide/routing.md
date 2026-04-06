---
title: Routing
description: Per-agent model and endpoint overrides via routing.agents in config.json.
summary: plan / explore / summary profiles and precedence vs global model settings.
read_when:
  - You want different models for planning vs exploration.
  - You are editing routing.agents for the first time.
---

# Routing

anyCode can override **model** and **endpoint-related fields** per **`agent_type`** so that, for example:

- **plan** uses a stronger model  
- **explore** uses a faster or cheaper model  
- **summary** uses a dedicated profile (or falls back to **plan**)

## Example

Edit **`~/.anycode/config.json`**:

```json
{
  "routing": {
    "agents": {
      "plan": { "model": "glm-5", "plan": "general" },
      "explore": { "model": "glm-4.7", "plan": "coding" },
      "summary": { "model": "glm-5", "plan": "general" }
    }
  }
}
```

## Precedence

1. **`routing.agents.<agent_type>`** when present  
2. **summary** stage: **`routing.agents.summary`** → **`routing.agents.plan`** → default  
3. Global **`model` / `plan` / `base_url`** from the root of **`config.json`**

## Related

- [Models](./models) — providers and **`plan`** field meaning  
- [CLI model](./cli-model) — interactive edits to **`routing.agents`**  
