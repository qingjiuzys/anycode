# Durable Task Diagnostics

Background nested agents remain process-local for execution, but they now write
a diagnostic state file when they start, finish, fail, or are cancelled:

```text
~/.anycode/tasks/<task-id>/state.json
```

The file is intentionally diagnostic-only for nested agents:

- It helps `TaskOutput`, future doctor commands, and users distinguish
  `running`, `completed`, `failed`, and `cancelled`.
- Nested agent `state.json` does **not** by itself resume tool loops after restart.

## Workflow DAG checkpoints (recoverable)

YAML / compiled workflows persist a **v2 checkpoint** under the project:

```text
<workdir>/.anycode/workflow-checkpoints/<workflow-name>.json
```

This records per-step status, artifact handoff summaries, gate hints, and
`depends_on` unlock state so Desktop/daemon can continue a DAG after restart
(see ADR 014). It is separate from nested-agent diagnostic `state.json`.

This remains inside the existing `AgentRuntime` authority — no second orchestrator.
