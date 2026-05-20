# Cron Production Notes

Cron has moved beyond reminder creation into production observability.

## Implemented

- `CronCreate` validates schedules before persistence.
- `schedule_timezone` supports `local`, `utc`, `utc0`, `gmt`, `Zulu`, and IANA
  names such as `Asia/Shanghai`.
- Scheduler writes `~/.anycode/logs/cron-runs.jsonl` with `job_id`, `session_id`,
  `fired_at`, `status`, and `detail`.
- `anycode cron runs` prints the run ledger with optional `--job`, `--session`,
  and `--limit` filters.
- New cron jobs persist a stable `session_id` correlation key (auto-generated
  when omitted).
- `CronCreate` accepts optional `failure_destination` (`log`, `same_channel`,
  `shell`, `http`) and `tool_profile` (`default`, `read_only`, `observability`).
- Scheduler enforces per-job tool profiles via `RunTaskOptions.tool_profile`.
- WeChat bridge routes failures to the last chat when
  `failure_destination = same_channel`.
- `failure_destination = shell` runs `ANYCODE_CRON_FAILURE_SHELL` with env vars
  `ANYCODE_CRON_JOB_ID`, `ANYCODE_CRON_SESSION_ID`, `ANYCODE_CRON_STATUS`, and
  `ANYCODE_CRON_ERROR` (detail truncated to 500 chars).
- `failure_destination = http` POSTs a short JSON payload to
  `ANYCODE_CRON_FAILURE_WEBHOOK`.

## Tool profiles

| Profile | Purpose |
|---------|---------|
| `default` | Full agent tool surface |
| `read_only` | Denies mutation tools and `mcp__*` |
| `observability` | Monitoring-only allowlist: read/search/list tools |
| `allowlist` | Custom explicit tool ids via `tool_allowlist` on the job |

## Next

- Use `session_id` to group recurring runs in stream UI and diagnostics.
