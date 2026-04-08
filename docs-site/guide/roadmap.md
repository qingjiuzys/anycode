---
title: Roadmap
description: MVP scope, acceptance scenarios, tools parity matrix, and post-MVP milestones for anyCode.
summary: What ships in MVP, how to validate it, P0–P8 tool stages, and MCP/LSP/Agent next steps.
read_when:
  - You plan releases or compare anyCode to other terminal agents.
  - You work on MCP, LSP, or sub-agent features.
---

# Roadmap

This page merges the former Chinese-only **MVP**, **tools-parity**, **roadmap-stubs**, and **MCP post-MVP** notes into one bilingual information architecture. **Source of truth for code** remains `crates/tools/src/catalog.rs`, `crates/tools/src/agent_tools.rs` (**Agent** / **Task**), `crates/cli/src/bootstrap/mod.rs`, and `crates/agent/src/agents.rs`.

## MVP scope (frozen)

**In MVP**

- Read/write/edit files, **Glob** / **Grep**, **Bash** (plus built-in tools documented as P1/P2/P7/P8 in the matrix below).  
- **Approval / sandbox** (`SecurityLayer` and config).  
- At least one practical **tool-calling** path for **z.ai (OpenAI-compatible)** and **Anthropic**.  
- **Execution logs** under **`~/.anycode/tasks/<id>/output.log`** and **summary** when not TUI-only output.  
- **CLI**: **`run`**, default TUI, **`repl`**, optional **`daemon` HTTP**.

**Out of MVP** (separate milestones; does not block an MVP release)

- Full **MCP** product story (SSE/HTTP, rich OAuth UI, lazy-loaded tools) beyond current stdio **v1** when enabled.  
- Full **LSP** subprocess story beyond experimental **`tools-lsp`**.  
- **Sub-agent:** full upstream parity for isolation/orchestration (**fork**, async **`run_in_background`**, etc.) remains a separate milestone; **worktree-level** isolation and Claude **`Agent` field** parity are described under **P5** below.  
- **Skill** plugin market / OpenClaw full parity beyond **`SKILL.md` + `Skill` tool** (see [Agent skills](./skills)).  
- **Swarm / coordinator**, plugin market, telemetry, voice, browser tools, etc.

When MVP boundaries change, update this section and the **acceptance** list below.

## MVP acceptance (suggested)

After each scenario, inspect **`~/.anycode/tasks/<task_id>/output.log`**:

- **`[task_start]`**, **`[turn_start]`** (or equivalent).  
- If tools should run: **`[tool_call_start]`** (or project convention — see [CLI sessions](./cli-sessions)).  
- Non-TUI paths: **`== summary ==`** or documented reason for no summary.

**A — Read-only:** Ask for `*.rs` glob, **Grep** a symbol, **FileRead** one file — expect matching tool calls and correct answer.

**B — Small write:** In a temp repo, add/edit a file with approval — expect **FileWrite** or **Edit** and disk matches instruction.

**C — Bash:** Harmless read-only command — **Bash** succeeds; sandbox still respected if enabled.

**D — Web:** **WebFetch** or **WebSearch** for a stable query — answer grounded in tool output.

**E — z.ai first-turn tools (recommended):** Set **`zai_tool_choice_first_turn`** or **`ANYCODE_ZAI_TOOL_CHOICE_FIRST_TURN=1`** — first request should include **tool_calls** when tools are required.

**F — Memory:** With **`memory.backend=file`**, seed a **`.md`** memory and ask a prompt that should inject **Relevant Memories** in the system prompt.

## Tools parity (P0–P8)

**Registry:** `crates/tools/src/catalog.rs` — `TOOL_*`, **`DEFAULT_TOOL_IDS`**, **`build_registry_with_services`**, **`validate_default_registry`**.  
**Bootstrap:** `crates/cli/src/bootstrap/mod.rs` — **`Arc<ToolServices>`** + registry build; **`SecurityLayer::set_tool_policy`** for sensitive tools.  
**Agent subsets:** `crates/agent/src/agents.rs` — **`general-purpose`** = full **`DEFAULT_TOOL_IDS`**; **`explore`** / **`plan`** = **`EXPLORE_PLAN_TOOL_IDS`** (still FileRead / Glob / Grep / Bash oriented).

**Naming (examples)**

| Reference name | API tool name | Notes |
|----------------|---------------|--------|
| FileEditTool | **Edit** | README may say FileEdit |
| SyntheticOutputTool | **StructuredOutput** | |
| BriefTool | **SendUserMessage** (alias **Brief**) | |
| MCPTool | **mcp** | lowercase |
| AgentTool | **Agent**; legacy **Task** | |

| Stage | Tools (API) | Status (high level) |
|-------|-------------|---------------------|
| P0 | Module split, `ToolServices`, registry build | Done |
| P1 | Edit, NotebookEdit, TodoWrite | Done |
| P2 | WebFetch, WebSearch | Done |
| P3 | mcp, ListMcpResourcesTool, ReadMcpResourceTool, McpAuth | **v1** with **`tools-mcp`**, **`ANYCODE_MCP_COMMAND`** / **`ANYCODE_MCP_SERVERS`**, deny rules, dynamic **`mcp__<server>__authenticate`** |
| P4 | LSP | Partial: **`tools-lsp`** + **`ANYCODE_LSP_COMMAND`** JSON-RPC forward; stub when off |
| P5 | Agent, Skill, SendMessage, Task (legacy) | **Skill v1** shipped; **Agent** / legacy **Task** run nested **`AgentRuntime`** (**`agent_type`** / **`subagent_type`**, nesting depth capped); Claude-style **`model`**, **`isolation: worktree`** (temp **git worktree**), **`cwd`** resolved to an **absolute** path; **`run_in_background: true`** is rejected (nested runs are synchronous); **SendMessage** stored in orchestration snapshot |
| P6 | TaskCreate/Update/List/Get/Stop/Output, team/cron/trigger | **v1** orchestration file **`~/.anycode/tasks/orchestration.json`** |
| P7 | Plan/worktree modes, ToolSearch, Sleep, StructuredOutput | Done |
| P8 | PowerShell, Config, Brief, AskUserQuestion, REPL | Done (PowerShell Windows-only) |

**Features:** `crates/tools/Cargo.toml` — **`tools-mcp`**, **`tools-lsp`** forwarded from the **`anycode`** package; **`tools-http`** reserved.

## Post-MCP / stub tracking

**MCP (beyond stdio v1)**

- Transports: stdio health checks; SSE/HTTP later.  
- Protocol: **`initialize`**, **`tools/list`**, **`tools/call`**, timeouts and errors in **`output.log`**.  
- Registration: single path **`initialize_runtime` → `build_registry_with_services`**.  
- Security: allow/deny for **`mcp__<server>__<tool>`**; sensitive calls through approval.  
- OAuth / **McpAuth**: align with dynamic tool names; clear errors without a GUI when servers require stdio OAuth.  
- Resources: real **List** / **Read** MCP resource tools.  
- Optional **lazy load** / **ToolSearch** integration later.

**Code entrypoints:** `mcp_normalization.rs`, `mcp_tools.rs`, `mcp_stdio.rs`, `bootstrap/mcp_env.rs`.

**LSP**

- Configurable subprocess: **`lsp_stdio.rs`** + **`tools-lsp`** feature.  
- **`ANYCODE_LSP_COMMAND`** enables JSON-RPC forwarding when built.

**P5 Agent / Skill**

- **Skill (shipped):** multi-root **`SKILL.md`** discovery, **`ToolServices.skill_catalog`**, system prompt **Available skills**, path-safe **`Skill`** execution (timeout, output cap, optional minimal env), config **`skills.*`**, CLI **`anycode skills list|path|init`**. Optional **`skills.expose_on_explore_plan`** registers **Skill** for **explore** / **plan**.  
- **Agent / legacy Task:** nested runs use the same **`AgentRuntime`** (**`SubAgentExecutor`**). **Claude Code `Agent` tool parity (current subset):** **`subagent_type`** (alias **`agent_type`**; **`Explore` / `Plan` / `general-purpose`** normalized), optional **`description`**, optional **`cwd`** (relative paths resolve against the tool-call working directory, then **canonicalized** to an absolute path), optional **`model`** (**`sonnet` / `opus` / `haiku`** or a raw model id, mapped for the session provider), optional **`isolation: "worktree"`** (**`git worktree add`** under the system temp dir, removed after the run). **`run_in_background: true`** returns a clear tool error (no async sub-agent queue yet). JSON responses echo **`model`** / **`isolation`** plus **`status`**, **`agent_id`** (= **`nested_task_id`**), **`output_file`**, and a Claude-style **`content`** text block on success. Still missing vs upstream: **fork-self**, true **async background** agents, teammate-swarm **`SendMessage`** semantics — later milestones.

**OpenAI official client**

- **`cargo build -p anycode --features openai`** — when **`provider`** is exactly **`openai`**, Chat Completions may use **`OpenAIClient`**; gateways often still use **`ZaiClient`**.

## Suggested next focus (maintainers)

Pick **one** primary thread for the next milestone-sized effort (avoid two large refactors in parallel). Open scoped GitHub issues per thread.

| Thread | Goal (issue-sized starters) |
|--------|-----------------------------|
| **P5 Agent / Task** | Optional: tighter alignment between **orchestration** records and daemon **`~/.anycode/tasks/<id>/`**; **fork** / true **async background** parity with Claude. |
| **MCP beyond stdio v1** | Stdio health checks and clearer errors; timeouts on `tools/call`; **McpAuth** / OAuth ergonomics without a GUI; real **List** / **Read** MCP resource tools. |

**Docs note:** explicit model-instructions file path is **only** via **`ANYCODE_MODEL_INSTRUCTIONS_FILE`**; JSON `model_instructions` controls **discovery** only — see [Config & security](./config-security).

## Related

- [Architecture](./architecture)  
- [Troubleshooting](./troubleshooting) — MCP / OAuth  

Chinese: [路线图](/zh/guide/roadmap).
