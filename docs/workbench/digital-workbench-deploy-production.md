# Digital Workbench — Production deploy checklist

**Audience:** Operators shipping the dashboard beyond a single developer laptop.  
**Scope:** Local-first SQLite bundle today; external Postgres/OIDC are V3+ roadmap items.

## Pre-flight

- [ ] Build release with embedded UI:  
  `ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode --features embedded-ui`
- [ ] Run full CI-equivalent checks: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`, dashboard-ui `npm test`, `npm run test:e2e`, and `python3 scripts/eval/run.py --with-mock`
- [ ] Confirm `anycode dashboard doctor` reports healthy DB + UI static path

## Runtime configuration

| Variable | Purpose | Default |
|----------|---------|---------|
| `ANYCODE_DASHBOARD_DB` | SQLite path | `~/.anycode/projects.db` |
| `ANYCODE_DASHBOARD_HOST` | Bind address | `127.0.0.1` |
| `ANYCODE_DASHBOARD_PORT` | Listen port | `43180` |
| `ANYCODE_DASHBOARD_RECORD` | CLI event recorder | `1` (set `0` on read-only replicas) |
| `ANYCODE_DASHBOARD_BLOCKED_ALERT_THRESHOLD` | Blocked-session alert | `0` |
| `ANYCODE_DASHBOARD_BASELINE_SESSION_MINUTES` | Saved-hours KPI baseline | `45` |
| `ANYCODE_DASHBOARD_HOURLY_RATE_USD` | Saved-hours value rate | `50` |
| `ANYCODE_TASK_TOKEN_BUDGET` | Optional default runtime token budget for REPL/nested paths | unset |
| `ANYCODE_TASK_COST_BUDGET_USD` | Optional default runtime cost budget for REPL/nested paths | unset |
| `ANYCODE_MCP_STRICT` | Require `ANYCODE_MCP_ALLOWED_TOOLS` for MCP calls | unset |
| `ANYCODE_MCP_ALLOWED_TOOLS` | Comma allowlist: `mcp__server__tool`, `tool`, or `server:tool` | unset |
| `ANYCODE_MCP_MAX_CALLS_PER_SERVER` | Per-process MCP call quota per server | unset |

## Security (local trusted model)

- [ ] Keep default bind on **loopback** unless API tokens are configured
- [ ] For non-loopback: create API token in Settings → enforce Bearer on all `/api/*`
- [ ] Do **not** commit connector tokens; use `GITHUB_TOKEN`, `LINEAR_API_KEY`, or env on the host
- [ ] Gate runner executes shell in project root — only register trusted project paths

## Process supervision

Example systemd unit (adjust paths):

```ini
[Unit]
Description=anyCode Digital Workbench
After=network.target

[Service]
Type=simple
User=anycode
Environment=ANYCODE_DASHBOARD_DB=/var/lib/anycode/projects.db
Environment=ANYCODE_DASHBOARD_HOST=127.0.0.1
Environment=ANYCODE_DASHBOARD_PORT=43180
ExecStart=/usr/local/bin/anycode dashboard --host 127.0.0.1 --port 43180
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Place nginx/Caddy in front if exposing beyond localhost; terminate TLS at the proxy.

## Data & backup

- [ ] SQLite WAL: copy `projects.db` + `-wal`/`-shm` together or use `anycode dashboard db backup`
- [ ] Schedule daily backup of `ANYCODE_DASHBOARD_DB` directory
- [ ] Document restore: stop service → replace DB files → start → run doctor
- [ ] Preview memory retention before pruning: `anycode memory prune --dry-run --older-than-days 90`

## Connectors (read-only POC)

- **GitHub:** connector JSON `{ "repo": "owner/name" }`, token via `GITHUB_TOKEN`
- **Linear:** `{ "team_key": "ENG" }` or `{ "team_id": "<uuid>" }`, token via `LINEAR_API_KEY`

No outbound write/sync — previews only.

## Observability

- [ ] Health: `GET /api/health`
- [ ] Doctor: `GET /api/settings/doctor` (bind, DB, UI dist, SSE)
- [ ] Audit trail: `GET /api/audit/events` for gate runs, policy changes, threshold alerts
- [ ] Export token usage CSV from Home or `GET /api/metrics/usage/export`

## Post-deploy smoke

1. Open Home — stats, token usage, saved-hours KPI load
2. Projects → pick workspace → run a gate preset → history row appears
3. Projects → **Trigger run** — detached subprocess starts; sensitive tools appear in the Web approval inbox when approval is required
4. Settings → doctor green; connectors list loads
5. `curl -s localhost:43180/api/bootstrap | jq .bootstrap.workbench_phase`
6. `./scripts/post-deploy-smoke.sh` (health / bootstrap / doctor / approval summary / overview)

## Included in this checklist

- UI-triggered agent runs (V3-W7) — loopback-only by default; uses the normal dashboard recorder and Web approval path
- Web tool approval inbox (V3-W8–W10)
- Live CLI cooperative cancel (V3-W6)

## Explicitly not in this checklist

SSO/RBAC · Postgres backend · connector write-back · Tauri desktop shell

See [production-harness-hardening.md](production-harness-hardening.md) for Tier 1.5 harness status and [digital-workbench-next-steps.md](../workbench/digital-workbench-next-steps.md) for Tier 2–3 roadmap.
