# `anycode-agent`

Agent **runtime**: multi-turn LLM + tool loop, task logging, and receipt/summary handling.

## Orchestration authority

**Use `AgentRuntime`**, not `Agent::execute`, for the main product path:

| API | Use when |
|-----|----------|
| `AgentRuntime::execute_task` | CLI `run`, nested sub-agent tasks, channel/scheduler-driven work |
| `AgentRuntime::execute_turn_from_messages` | Fullscreen TUI (caller owns `messages` history) |

The **`Agent`** trait still matters for **which tools** are advertised (`tools()`), **agent metadata**, and **system prompt** composition. See [ADR 000](../../docs/adr/000-runtime-orchestration.md).

## Layout

- `agents.rs` — built-in `GeneralPurposeAgent`, `ExploreAgent`, `PlanAgent`
- `runtime/` — `AgentRuntime`, tool loop, `tool_surface` (tool names + schemas for the LLM), logging, summaries; `runtime_options.rs` groups constructor args (`RuntimeCoreDeps`, …); `tool_gating.rs` holds `AgentClaudeToolGating`
- `system_prompt.rs` — effective system prompt from config + agent + memory

## Tests

```bash
cargo test -p anycode-agent
```
