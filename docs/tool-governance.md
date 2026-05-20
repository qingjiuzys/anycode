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
- Runtime tool policy profiles for headless, CI, and channel surfaces
  (`crates/tools/src/runtime_tool_policy.rs`, wired from `crates/cli/src/tool_policy.rs`).
- Operator-facing audit query CLI: `anycode audit tail`.

### Runtime tool policy profiles

Named profiles reuse the same deny lists as cron (`read_only`, `observability`, etc.).
Resolution order: per-job/cron `tool_profile` → `ANYCODE_TOOL_PROFILE` env →
config `runtime.tool_policy_profiles.<surface>` → built-in defaults (`ci` →
`read_only`, `channel` → `observability`). Additive env/config denies merge on top.

| Surface | When applied | Default profile (if unset in config) |
|---------|--------------|--------------------------------------|
| `headless` | `anycode run`, scheduler (no per-job profile) | none |
| `ci` | `CI=true` / `GITHUB_ACTIONS=true` on headless runs | `read_only` |
| `channel` | WeChat / Telegram / Discord bridges | `observability` |

Config (`~/.anycode/config.json`):

```json
{
  "runtime": {
    "tool_policy_profiles": {
      "headless": "read_only",
      "ci": "read_only",
      "channel": "observability"
    },
    "tool_deny_names": [],
    "tool_deny_prefixes": []
  }
}
```

Environment overrides (process-wide, merged after profile):

| Variable | Effect |
|----------|--------|
| `ANYCODE_TOOL_PROFILE` | Force named profile (`default`, `read_only`, `observability`, `allowlist`) |
| `ANYCODE_TOOL_DENY` | Comma-separated tool names to hide from the LLM |
| `ANYCODE_TOOL_DENY_PREFIXES` | Comma-separated prefixes (e.g. `mcp__`) |

## Next

- Graph memory retrieval path (ADR 009; evidence JSONL remains SSOT until then).
- Eval harness tasks with repository fixtures (SWE-bench-lite style).
