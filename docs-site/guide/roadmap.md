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

**Maintainer execution backlog** (now / next / later, decisions): repository **[`docs/roadmap.md`](https://github.com/qingjiuzys/anycode/blob/main/docs/roadmap.md)** — edit that file instead of duplicating task lists here.

## MVP scope (frozen)

**In MVP**

- Read/write/edit files, **Glob** / **Grep**, **Bash** (plus built-in tools documented as P1/P2/P7/P8 in the matrix below).  
- **Approval / sandbox** (`SecurityLayer` and config).  
- At least one practical **tool-calling** path for **z.ai (OpenAI-compatible)** and **Anthropic**.  
- **Execution logs** under **`~/.anycode/tasks/<id>/output.log`** and **summary** when not TUI-only output.  
- **CLI**: **`run`**, **`repl`**, **`tui`**, channel bridges, **`scheduler`**, etc. (**HTTP `daemon`** was removed — see [ADR 003](https://github.com/qingjiuzys/anycode/blob/main/docs/adr/003-http-daemon-deprecated.md) and [HTTP daemon (removed)](./cli-daemon).)

**Out of MVP** (separate milestones; does not block an MVP release)

- Full **MCP** product story (SSE/HTTP, rich OAuth UI, lazy-loaded tools) beyond current stdio **v1** when enabled.  
- Full **LSP** subprocess story beyond experimental **`tools-lsp`**.  
- **Sub-agent:** full upstream parity for isolation/orchestration (**fork**, durable cross-process background agents, etc.) remains a separate milestone; **worktree-level** isolation, **`run_in_background` v1** (in-process registry + **`TaskOutput`** / **`TaskStop`**), and Claude **`Agent` field** parity are described under **P5** below.  
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
| P5 | Agent, Skill, SendMessage, Task (legacy) | **Skill v1** shipped; **Agent** / legacy **Task** run nested **`AgentRuntime`** (**`agent_type`** / **`subagent_type`**, nesting depth capped); Claude-style **`model`**, **`isolation: worktree`** (temp **git worktree**), **`cwd`** resolved to an **absolute** path; **`run_in_background: true`** returns **`status: started`** and runs nested work in a background task — poll **`TaskOutput`**, cancel with **`TaskStop`** on **`nested_task_id`** (cooperative flag at nested turn/tool boundaries + **`AbortHandle`** fallback; in-process only); **SendMessage** stored in orchestration snapshot |
| P6 | TaskCreate/Update/List/Get/Stop/Output, team/cron/trigger | **v1** orchestration file **`~/.anycode/tasks/orchestration.json`** |
| P7 | Plan/worktree modes, ToolSearch, Sleep, StructuredOutput | Done |
| P8 | PowerShell, Config, Brief, AskUserQuestion, REPL | Done (PowerShell Windows-only) |

**Features:** `crates/tools/Cargo.toml` — **`tools-mcp`**, **`tools-lsp`** forwarded from the **`anycode`** package; **`tools-http`** reserved.

## Post-MCP / stub tracking

**MCP (beyond stdio v1)**

- Transports: stdio **`ANYCODE_MCP_READ_TIMEOUT_SECS`** (JSON-RPC line read), clearer timeout/EOF errors, **`McpStdioSession::stdio_child_is_running`**; SSE/HTTP later.  
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
- **Agent / legacy Task:** nested runs use the same **`AgentRuntime`** (**`SubAgentExecutor`**). **Claude Code `Agent` tool parity (current subset):** **`subagent_type`** (alias **`agent_type`**; **`Explore` / `Plan` / `general-purpose`** normalized), optional **`description`**, optional **`cwd`** (relative paths resolve against the tool-call working directory, then **canonicalized** to an absolute path), optional **`model`** (**`sonnet` / `opus` / `haiku`** or a raw model id, mapped for the session provider), optional **`isolation: "worktree"`** (**`git worktree add`** under the system temp dir, removed after the run). **`run_in_background: true`** returns immediately with **`status: started`**, the same **`nested_task_id`** / **`output_file`** hints as sync runs, and a process-local registry; use **`TaskOutput`** for **`background_status`** / log tail and **`TaskStop`** on the UUID: a shared flag is polled at nested **turn** and **tool** boundaries **and** during blocking **`chat` / streaming** (`tokio::select!`, short-interval poll—no hard guarantee the TCP stream stops immediately). **`AbortHandle::abort`** remains fallback; syscall-blocking tools may still need abort. Sync success JSON still includes **`content`**. Still missing vs upstream: **fork-self**, durable cross-process background agents, teammate-swarm **`SendMessage`** semantics — later milestones.

**OpenAI official client**

- **`cargo build -p anycode --features openai`** — when **`provider`** is exactly **`openai`**, Chat Completions may use **`OpenAIClient`**; gateways often still use **`ZaiClient`**.

## Recently shipped (reference)

- **Nested-task cooperative cancel (v2 + v2.1):** **`TaskStop`** sets a flag wired to nested **`TaskContext`** / TUI **`coop_cancel`**; **`tokio::select!`** competes with **`chat`**, **`chat_stream`**, and stream **`recv`** (~20ms poll). **`TaskResult::Failure`** **`cancelled`** maps to **`background_status: cancelled`**. **Optional later:** **`tokio_util::sync::CancellationToken`**; **syscall-blocked** tools stay **best-effort** (**`AbortHandle`**).

## Suggested next focus (maintainers)

Pick **one** primary thread for the next milestone-sized effort (avoid two large refactors in parallel). Open scoped GitHub issues per thread.

| Thread | Goal (issue-sized starters) |
|--------|-----------------------------|
| **P5 Agent / Task** | Optional: tighter alignment between **orchestration** records and task **`~/.anycode/tasks/<id>/`** layouts; **fork-self**; durable cross-process background agents (beyond in-process registry). |
| **MCP beyond stdio v1** | **Partial:** **`ANYCODE_MCP_READ_TIMEOUT_SECS`** overrides JSON-RPC per-line read timeout; clearer timeout / unexpected-EOF errors (incl. child exit); **`McpStdioSession::stdio_child_is_running`** for health checks. **Still open:** richer stdio lifecycle / reconnect; **McpAuth** / OAuth ergonomics without a GUI; polish **List** / **Read** MCP resource tools in UX. |
| **Channel AskUserQuestion** | Card / inline-keyboard flows for WeChat / Telegram / Discord (new host implementations). |

**Docs note:** explicit model-instructions file path is **only** via **`ANYCODE_MODEL_INSTRUCTIONS_FILE`**; JSON `model_instructions` controls **discovery** only — see [Config & security](./config-security).

## Related

- [Architecture](./architecture)  
- [Troubleshooting](./troubleshooting) — MCP / OAuth  

Chinese: [路线图](/zh/guide/roadmap).
