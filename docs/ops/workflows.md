# AnyCode Workflows

`workflow.yml` or `workflow.yaml` can live in the workspace root or `.anycode/`.

Minimal schema:

```yaml
name: ship-feature
mode: goal
model: best
steps:
  - id: inspect
    prompt: Inspect the codebase and summarize the current state.
  - id: implement
    prompt: Implement the requested change.
retry:
  max_attempts: 5
  backoff_ms: 1000
done_when: tests pass and the requested feature works
handoff:
  next_mode: channel
  message: Report completion back to the workspace assistant.
```
