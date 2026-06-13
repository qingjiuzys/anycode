# Digital Workbench — MVP completion checklist

Status: **V1 acceptance complete** + **V2 slices A–D complete** — see [`digital-workbench-v2-complete.md`](digital-workbench-v2-complete.md).

## UX acceptance (7)

| # | Criterion | Status |
|---|-----------|--------|
| 1 | `anycode dashboard` opens UI | **Done** — `embedded-ui` feature embeds `dashboard-ui/dist` in release; CI builds UI before release |
| 2 | Select project | **Done** |
| 3 | Recent sessions | **Done** |
| 4 | Inspect goal/run | **Done** — replay panel, SSE, user/assistant events |
| 5 | Why not trusted | **Done** — gate panel; completed run/repl without gates → verified |
| 6 | Artifacts + gate | **Done** — FileWrite/Edit/Bash redirect paths; auto-verify on passed gates |
| 7 | Port + SQLite prefs | **Done** — saves prefs; restart documented |

## Engineering

- SQLite migrations + recorder → API → React UI
- Playwright smoke: `crates/dashboard-ui/e2e/acceptance.spec.ts`
- CI: Rust + dashboard-ui build/test/e2e
- Token usage on Home (`/api/metrics/usage`) + **estimated USD cost**
- Notifications feed (`/api/notifications/recent`)
- Connectors: read-only UI (no CRUD)
- Code-split vendor chunks (echarts, reactflow, react)

## V2 (complete)

- Per-project token usage + CSV export + blocked-threshold alert
- GitHub connector open-issues POC (Settings + Automations)
- Gate runner (presets, execute, persist to gates/events)
- `install-with-dashboard.sh` + docs-site V2 API

## Out of scope (V3+)

See **`docs/digital-workbench-handoff.md`** or **`docs/workbench/digital-workbench-next-steps.md`** for roadmap.

## Verify locally

```bash
ANYCODE_BUILD_DASHBOARD_UI=1 ./scripts/build-dashboard-ui.sh
cargo build --release -p anycode --features embedded-ui
anycode dashboard --open
cd crates/dashboard-ui && npm run test:e2e
```
