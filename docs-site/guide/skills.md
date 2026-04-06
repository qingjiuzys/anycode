---
title: Agent skills
description: SKILL.md layout, ~/.anycode/skills discovery, config skills.*, Skill tool, and anycode skills CLI.
summary: How anyCode discovers skills, injects them into the system prompt, and runs optional run scripts.
read_when:
  - You want folder layout and config for Agent Skills–style extensions.
  - You need CLI commands to list roots or scaffold a skill.
---

# Agent skills

anyCode aligns with common **Agent Skills** conventions: each skill is a directory with a **`SKILL.md`** file whose YAML frontmatter includes **`name`** and **`description`**. An optional executable **`run`** in that directory is invoked by the **`Skill`** tool (same broad risk class as **Bash** — approvals / sensitive-tool policy apply).

## Layout

- **User-wide default root:** `~/.anycode/skills/<skill_id>/`
- **Project overrides (no startup scan):** `<cwd>/skills/<skill_id>/` or `<cwd>/.anycode/skills/<skill_id>/` — resolved when the **Skill** tool runs if the id is not already in the catalog.
- **`skill_id`** must match the directory name and the frontmatter **`name`** (ASCII letters, digits, `.`, `_`, `-` only). Mismatches are skipped with a log warning.

Minimal **`SKILL.md`**:

```markdown
---
name: my-skill
description: One line for the model and for `anycode skills list`.
---

# my-skill

Longer documentation for humans (optional).
```

Optional **`run`** (e.g. bash): must be a regular file; executed with the skill directory as **cwd**, with optional CLI args passed through.

## Config (`~/.anycode/config.json`)

Under **`skills`**:

| Field | Meaning |
|-------|---------|
| **`enabled`** | When `true`, scan **`skills.extra_dirs`** then **`~/.anycode/skills`** at startup; build catalog and inject **## Available skills** into the default system stack (skipped when **`system_prompt_override`** is set). |
| **`extra_dirs`** | Extra scan roots (lower precedence than **`~/.anycode/skills`**; later roots override same id). |
| **`allowlist`** | If set, only these ids appear in the catalog and prompt. |
| **`run_timeout_ms`** | Subprocess timeout for **`run`** (minimum enforced in code). |
| **`minimal_env`** | When `true`, only a small env whitelist (**PATH**, **HOME**, **USER**, etc.) is passed to **`run`**. |
| **`expose_on_explore_plan`** | When `true` **and** **`enabled`**, **explore** / **plan** agents also get the **Skill** tool (default `false` to limit code execution surface). |

## CLI

```bash
anycode skills list   # id, has run, description, root
anycode skills path   # effective scan roots + skills.enabled
anycode skills init <name>   # ~/.anycode/skills/<name>/ + SKILL.md + run template
```

## Model visibility

When skills are enabled and the default system prompt stack is used, the prompt includes an **Available skills** section listing ids and descriptions. Execution still goes through the **Skill** tool with **`{"name": "<id>", "args": [...]}`**.

## Related

- [Config & security](./config-security) — approvals and sandbox  
- [Discovery & test-security](./cli-diagnostics) — **`list-tools`**  
- [Architecture](./architecture) — bootstrap and **ToolServices**  

Chinese: [Agent skills（中文）](/zh/guide/skills).
