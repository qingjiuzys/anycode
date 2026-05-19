# Cron observability (builtin scheduler)

The embedded scheduler (`run_builtin_scheduler`, WeChat/Telegram/Discord bridges) appends one JSON line per job fire under:

`~/.anycode/logs/cron-runs.jsonl`

## Record shape

Each line is a JSON object:

| Field | Meaning |
|-------|---------|
| `job_id` | Id returned by `CronCreate` |
| `fired_at` | RFC3339 UTC timestamp for the scheduled fire |
| `status` | `started`, `ok`, or `error` |
| `detail` | Error message, or first 200 chars of agent output on success |

## Related

- `CronCreate` rejects invalid cron expressions before persisting (`validate_cron_schedule_expr`).
- `schedule_timezone`: only `local` (default) and `utc` are supported; IANA zones are not implemented yet.
- [roadmap.md](roadmap.md) §4 automation row
