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

## Workspace overlay vs global defaults

When the CLI loads config it may apply **`workspace::apply_project_overlays`**: if the current directory matches an entry in **`~/.anycode/workspace/projects/index.json`**, project **`default_mode`**, **`label`**, and **`channel_profile`** override the in-memory **`Config`** for that process (they do not rewrite **`config.json`**). Global **`config.json`** remains the source for **`provider`**, **`model`**, and **`routing`**.

## `runtime.model_routes` (mode / agent aliases)

Optional map in **`config.json`** under **`runtime.model_routes`**: **`mode_aliases`** keys are **`RuntimeMode`** strings (`general`, `plan`, `code`, `explore`, `channel`, `goal`). Values are built-in alias names (`best`, `fast`, `plan`, `code`, `channel`, `summary`) or per-agent overrides. Documented defaults that match the built-in router are available in code as **`ModelRouteProfile::documented_mode_alias_defaults()`** (for templates and docs).

## YAML workflows

File-based workflows (`workflow.yml`, `.anycode/workflow.yaml`) complement routing: see the example **[workflow.example.yml](https://github.com/qingjiuzys/anycode/blob/main/examples/workflow.example.yml)** in the repo.

## Related

- [Models](./models) — providers and **`plan`** field meaning  
- [CLI model](./cli-model) — interactive edits to **`routing.agents`**  
