# Durable Task Diagnostics

Background nested agents remain process-local for execution, but they now write
a diagnostic state file when they start, finish, fail, or are cancelled:

```text
~/.anycode/tasks/<task-id>/state.json
```

The file is intentionally diagnostic-only:

- It helps `TaskOutput`, future doctor commands, and users distinguish
  `running`, `completed`, `failed`, and `cancelled`.
- It does **not** promise execution recovery after process restart.
- Future cross-process agents must add a separate ADR before resuming work from
  this state file.

This is the first production-safe step toward durable background agents without
creating a second orchestration engine outside `AgentRuntime`.

