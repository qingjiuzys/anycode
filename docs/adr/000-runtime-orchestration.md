# ADR 000: AgentRuntime as the sole orchestration authority

## Status

Accepted

## Context

anyCode exposes an **`Agent`** trait in `anycode-core` with an **`execute`** method, while the CLI and TUI (and other supported entrypoints such as **`run`**) actually run tasks through **`AgentRuntime::execute_task`** and **`execute_turn_from_messages`**. Contributors can assume the wrong entry point unless the rule is documented.

## Decision

1. **Orchestration authority**: Multi-turn LLM calls, tool execution, logging, and (where applicable) summary generation are implemented **only** in **`anycode_agent::AgentRuntime`** (`execute_task` for one-shot tasks, `execute_turn_from_messages` for TUI sessions).
2. **`Agent` trait role**: Supplies **agent type**, **tool name subset** (`tools()`), **description**, and **system prompt** hooks. The default **`Agent::execute`** implementations are **not** invoked by the current CLI/TUI main paths.
3. **Extensions**: New capabilities should extend **`Tool`** + **`build_registry_with_services`** + CLI **`bootstrap`**, not a second parallel “runner” trait hierarchy.

## Consequences

- Documentation and onboarding must point to **`AgentRuntime`** first; see `crates/agent/README.md` and the docs-site architecture pages.
- If a future mode truly needs **`Agent::execute`**, that should be a deliberate ADR amendment with call sites listed.

## Related

- `anycode-core`: `Agent` trait rustdoc
- `crates/agent/src/runtime/mod.rs`
- `docs-site/guide/architecture.md`
