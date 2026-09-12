# ADR 000: AgentRuntime as the sole orchestration authority

## Status

Accepted (current). Amended 2026-09-11 for the harness-v1 Kernel adapters.

## Context

anyCode exposes an **`Agent`** trait in `anycode-core` with an **`execute`** method, while the CLI and TUI (and other supported entrypoints such as **`run`**) actually run tasks through **`AgentRuntime::execute_task`** and **`execute_turn_from_messages`**. Contributors can assume the wrong entry point unless the rule is documented.

Multi-step workflow DAGs (Workbench graph runs, cron orchestration) need a coordinator that is **not** a second ReAct loop.

## Decision

1. **Orchestration authority**: Multi-turn LLM calls, tool execution, logging, and (where applicable) summary generation are implemented **only** in **`anycode_agent::AgentRuntime`**. Default chat and scheduler still use the legacy `execute_task` / `execute_turn_from_messages` loops. When the opt-in `harness-v1-unified-kernel` flag is armed, those two entries become thin adapters over the same **`harness-core` Kernel** (Host still calls `execute_tool_call`). The Kernel must not be nested inside the legacy loop on the same task. Graph nodes and Supervisor children use that Kernel directly. The product `Agent` tool on the unified parent is bound only so it can reach Supervisor through `execute_tool_call`; pilot / graph / child hosts still reject `Agent` / `Task` bindings. Children inherit the parent `TaskBudget` / `BudgetPool` instead of reading process environment. Unified Kernel refuses detached `run_in_background` `tokio::spawn`.
2. **`Agent` trait role**: Supplies **agent type**, **tool name subset** (`tools()`), **description**, and **system prompt** hooks. The default **`Agent::execute`** implementations are **not** invoked by the current CLI/TUI main paths.
3. **GraphEngine (multi-step DAG)**: **`anycode_agent::GraphEngine`** remains the legacy coordinator and still uses `POST /api/sessions/{id}/graph/run`. The harness GraphRunner is a separate product surface (`POST /api/projects/{id}/harness/graphs/start|resume|resolve`). Start and Resume are different APIs. Work nodes on that surface use the same Kernel host; they do not invent a second loop. Sample verifier IDs stay unsupported; `artifact.sha256` hashes a host-owned evidence file.
4. **Extensions**: New capabilities should extend **`Tool`** + **`build_registry_with_services`** + CLI **`bootstrap`**, not a second parallel “runner” trait hierarchy.

## Consequences

- Documentation and onboarding must point to **`AgentRuntime`** first; see `crates/agent/README.md` and [user architecture guide](https://anycode.work/docs/guide/architecture).
- Graph/workflow features must not re-implement tool loops inside dashboard handlers — legacy routes delegate to **`GraphEngine::run`**; harness routes delegate to **`GraphRunner`** + Kernel.
- If a future mode truly needs **`Agent::execute`**, that should be a deliberate ADR amendment with call sites listed.

## Related

- `anycode-core`: `Agent` trait rustdoc
- `crates/agent/src/runtime/mod.rs`
- `crates/agent/src/graph_engine.rs`
- `https://anycode.work/docs/guide/architecture`
