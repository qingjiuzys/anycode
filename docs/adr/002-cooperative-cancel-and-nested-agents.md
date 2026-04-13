# ADR 002: Cooperative cancel — main session, turns, and nested agents

## Status

Accepted

## Context

anyCode uses **cooperative cancellation** (not POSIX signal injection into the LLM client): an `Arc<AtomicBool>` is set while a turn runs; `AgentRuntime` polls / `select!`s on it and ends the turn with a dedicated outcome. Paths include:

- **Full-screen TUI / stream REPL**: `turn_coop_cancel` passed into `execute_turn_from_messages`.
- **Non-TTY line REPL**: Ctrl+C sets the same flag (no ratatui key path).
- **Nested `execute_task`**: `TaskContext.nested_cancel` is populated from `NestedTaskInvoke.cancel` when the Claude-style **Agent** tool runs a sub-agent.

Background nested agents register a per-job flag; **TaskStop** sets that flag and aborts the spawned task. Earlier JSON notes suggested nested runs had “no” cooperative token; that was misleading for the **background** path.

## Decision

1. **Single semantic outcome**: Cooperative cancel for turns is reported as [`CoreError::CooperativeCancel`](../../crates/core/src/error.rs) (display: `LLM error: cancelled`, matching the legacy `LLMError("cancelled")` string). Callers should use `CoreError::is_cooperative_cancel()` which also treats the legacy `LLMError` form for compatibility.
2. **Nested foreground**: `NestedTaskInvoke.cancel` is often `None`; cancellation is not wired unless a caller supplies a flag.
3. **Nested background**: `invoke.cancel` is set to the job’s `coop_cancel`; **TaskStop** sets the flag and best-effort aborts the tokio task. The nested `execute_task` loop honors `nested_cancel` at turn and tool boundaries (same mechanism as main-session turn cancel).
4. **Documentation**: User-facing architecture pages link here; **TaskStop** tool JSON should describe **best-effort** abort (flag + task abort), not “no cooperative cancel”.

## Consequences

- UI layers should prefer `is_cooperative_cancel` / downcast `anyhow::Error` to `CoreError` instead of comparing formatted strings.
- If a future mode needs **foreground** nested cancel from the parent session, thread an optional `Arc<AtomicBool>` from the parent into `NestedTaskInvoke.cancel` (same field), rather than adding a second mechanism.

## Related

- `crates/core/src/task.rs` — `NESTED_TASK_COOPERATIVE_CANCEL_ERROR` for `TaskResult::Failure` strings and background status.
- `crates/agent/src/runtime/mod.rs` — `execute_turn_from_messages` / `execute_task` cooperative paths.
- ADR 000 — orchestration authority remains `AgentRuntime`.
