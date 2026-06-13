# Cron observability (builtin scheduler)

The embedded scheduler (`run_builtin_scheduler`, WeChat/Telegram/Discord bridges) appends one JSON line per job fire under:

`~/.anycode/logs/cron-runs.jsonl`

## Record shape

Each line is a JSON object:

| Field | Meaning |
|-------|---------|
| `job_id` | Id returned by `CronCreate` |
| `session_id` | Stable cron session correlation id (when set on the job) |
| `fired_at` | RFC3339 UTC timestamp for the scheduled fire |
| `status` | `started`, `ok`, or `error` |
| `detail` | Error message, or first 200 chars of agent output on success |

Query the ledger:

```bash
anycode cron runs --job <id> --session <session_id> --limit 20 --json
```

## Failure routing

When a cron fire fails (`status: error`), `failure_destination` on the job controls
where a short sanitized summary is sent:

| Value | Behavior |
|-------|----------|
| `log` (default) | Ledger only |
| `same_channel` | WeChat bridge pushes to the last chat |
| `shell` | Runs `ANYCODE_CRON_FAILURE_SHELL` with env vars `ANYCODE_CRON_*` |
| `http` | POST JSON to `ANYCODE_CRON_FAILURE_WEBHOOK` |

Details are truncated to 500 characters and must not include raw tool output.

## Related

- `CronCreate` rejects invalid cron expressions before persisting (`validate_cron_schedule_expr`).
- `schedule_timezone`: `local` (default), `utc` / `utc0` / `gmt`, or an **IANA** name (e.g. `Asia/Shanghai`) for wall-clock conversion before UTC storage.
- **`CronCreate` response** includes `next_fire_utc` and `next_fire_local` when the expression parses (recurring or one-shot after storage conversion). Use these to confirm IM-scheduled reminders before the first scheduler tick.
- [roadmap.md](../roadmap.md) §4 automation row
