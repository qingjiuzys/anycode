---
title: Agents & Skills
description: Built-in agent types, declarative profiles, skills governance, and database persistence.
summary: Five built-in agents, custom profiles via config.json, project_skills enforcement, Dashboard CRUD.
read_when:
  - You want to customize agent tool sets or skills bindings.
  - You are configuring agents.profiles or routing.agents.
---

# Agents & Skills

anyCode separates **orchestration** (single `AgentRuntime`) from **agent profiles** (tool surface + prompts + routing).

## Built-in agents

Registered in the Rust runtime (cannot be deleted):

| `agent_type` | Role |
|--------------|------|
| `general-purpose` | Full default tool set + `Skill` tool |
| `explore` | Read/search tools; optional `Skill` when `skills.expose_on_explore_plan` |
| `plan` | Same as explore, planning-oriented prompt |
| `workspace-assistant` | Channel/cron-oriented subset |
| `goal` | Full tools for autonomous goal loops |

`summary` is a **routing key** for compaction only — not a registered agent.

## Shipped role profiles

These extend built-ins and are always available at runtime (unless you define the same id in config):

`builder`, `planner`, `explorer`, `verifier`, `reviewer`, `channel-ops`, `goal-runner`

## Custom profiles (`config.json`)

Add declarative agents under `agents.profiles`:

```json
{
  "agents": {
    "profiles": {
      "reviewer": {
        "extends": "explore",
        "description": "PR review without shell",
        "tools": { "allow": ["FileRead", "Grep", "Glob"] },
        "skills": { "allowlist": ["code-review"] },
        "routing": { "model": "glm-4.7", "plan": "coding" }
      }
    },
    "defaults": {
      "run": "general-purpose",
      "goal": "goal",
      "channel": "workspace-assistant"
    }
  }
}
```

CLI: `anycode run --agent reviewer`

## Model routing

Per-agent LLM overrides live in `routing.agents` (Dashboard **Settings → Model & routing**). Profile-level `routing` merges when no explicit `routing.agents` entry exists.

## Skills governance

Effective skills for an agent:

```
global allowlist ∩ agent allowlist ∩ project_skills.enabled
```

- **Global**: `skills.allowlist` in config
- **Per agent**: `skills.agent_allowlists` or `agents.profiles.*.skills.allowlist`
- **Project**: `project_skills` in `~/.anycode/projects.db` (Dashboard Skills page)

The `Skill` tool rejects ids outside the effective set.

## Database

| Store | Purpose |
|-------|---------|
| `~/.anycode/config.json` | SSOT for profiles + routing |
| `~/.anycode/projects.db` → `agent_profiles` | Dashboard mirror + CRUD |
| `sessions.agent_type` | Usage stats (not profile definitions) |

Legacy `agents` table (builder role columns) is unused; use `agent_profiles` instead.

## Dashboard

- **Agent / Skills** page: usage stats + configure links
- **Settings → Agents**: CRUD custom profiles (syncs config + DB)
- **Conversation start**: optional agent picker

## Related

- [Routing](./routing) — `routing.agents` model overrides
- [Models](./models) — provider catalog and probes
