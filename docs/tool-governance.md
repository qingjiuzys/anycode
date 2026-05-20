# Tool Governance

anyCode writes a best-effort audit stream for tool calls:

```text
~/.anycode/audit/tool-calls.jsonl
```

Each row records:

- `task_id`
- `phase` (`pre_check`, `policy`, `approval`, `execute`, `result`)
- `tool_name`
- `working_directory`
- `input_hash` (stable hash of the JSON input, not the raw input)
- `outcome`
- optional `detail`

The audit writer is deliberately non-blocking from the product perspective:
failure to create or append the log never changes tool execution behavior. This
keeps the security boundary in `SecurityLayer` while adding an operational trail
for production debugging.

## Implemented hardening

- MCP `tools/list` scanner flags suspicious tool descriptions and schemas
  (`crates/tools/src/mcp_tool_scan.rs`, wired when `tools-mcp` is enabled).
- Tool output sanitizer metadata for WebFetch, MCP, and Bash outputs
  (`crates/agent/src/runtime/tool_output_sanitize.rs`).
- Per-cron tool profiles (`default`, `read_only`, `observability`, `allowlist`) enforced at
  task execution via `TaskContext.tool_deny_names` / `tool_deny_prefixes`.
- Operator-facing audit query CLI: `anycode audit tail`.

## Next

- Policy profiles for headless / CI / channel modes beyond cron profiles.
