# Digital Workbench — V2 completion checklist

Status: **V2 slices A–D complete** for local use (2026-05). Builds on V1 MVP.

## Slice A — Observability

| Item | Status |
|------|--------|
| Per-project token/cost on project detail | **Done** — `GET /api/projects/{id}/usage` + `ProjectTokenUsage` |
| Export usage CSV | **Done** — `GET /api/metrics/usage/export` + Home export button |
| Blocked sessions > N alert | **Done** — `ANYCODE_DASHBOARD_BLOCKED_ALERT_THRESHOLD` + `blocked_threshold_exceeded` notification |

## Slice B — Connector POC

| Item | Status |
|------|--------|
| GitHub read-only open issues | **Done** — `GET /api/settings/connectors/{id}/github/issues` |
| Surface in Settings / Automations / project context | **Done** — `GitHubIssuesPanel` on Settings + Automations (per project) |

## Slice C — Gate runner

| Item | Status |
|------|--------|
| Presets (cargo/npm/playwright/flutter) | **Done** — `gate_runner.rs` + `GET .../gates/presets` |
| Execute from UI | **Done** — `POST .../gates/execute` + `GateRunnerPanel` |
| Persist to gates table + timeline | **Done** — `manual_gate` session + `upsert_gate` + `gate_executed` event |
| Goal engine real verify | **Done (V1)** — `goal_engine` runs `cargo test` / `flutter test` / README marker checks; logs `[gate]` lines ingested by recorder |

## Slice D — Packaging

| Item | Status |
|------|--------|
| Install script with embedded UI | **Done** — `--with-dashboard`, `scripts/install-with-dashboard.sh` |
| docs-site user guide | **Done** — en/zh `guide/dashboard.md` V2 API + env table |

## Verify

```bash
ANYCODE_BUILD_DASHBOARD_UI=1 ./scripts/build-dashboard-ui.sh
cargo test -p anycode-dashboard
cd crates/dashboard-ui && npm test && npm run test:e2e
ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode --features embedded-ui
anycode dashboard --open
```

## Explicitly still out of scope (V3+)

- OAuth / write-back for GitHub/Linear
- SSO / multi-user RBAC
- UI → start/cancel agent runs
- Headless browser `done_when` rules beyond goal engine shell checks
- Tauri desktop shell
- Per-provider model pricing catalog
- Saved-hours KPI

See [`digital-workbench-handoff.md`](digital-workbench-handoff.md) for planning.

**Start here for V3 planning:** [`digital-workbench-next-steps.md`](digital-workbench-next-steps.md)
