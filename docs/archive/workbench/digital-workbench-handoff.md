# Digital Workbench — Handoff & Next Planning

**Status:** V1 MVP + **V2 slices A–D complete** (2026-05). One-page status: [`digital-workbench-STATUS.md`](digital-workbench-STATUS.md). Use this doc to plan V3+.

Completion checklists: [`digital-workbench-v1-mvp.md`](digital-workbench-v1-mvp.md), [`digital-workbench-v2-complete.md`](digital-workbench-v2-complete.md).

## What shipped

### V1 (MVP)

| Area | Delivered |
|------|-----------|
| **CLI** | `anycode dashboard` (+ doctor, status, token, db) |
| **Data** | SQLite `~/.anycode/projects.db`, recorder from run/goal/workflow/repl/cron |
| **Trust** | Gates → `trusted_status`; completed run/repl without gates → verified |
| **UI** | React/Vite, i18n zh/en, SSE, 12+ pages, lazy routes, code-split vendors |
| **Release** | `embedded-ui` embeds `dashboard-ui/dist` in release binary |
| **Tests** | Rust unit/integration, Vitest i18n parity, Playwright UX smokes |
| **CI** | Rust + dashboard-ui build/test/e2e |

### V2 (local polish)

| Slice | Delivered |
|-------|-----------|
| **A Observability** | Per-project usage, CSV export, blocked-threshold alert |
| **B Connector POC** | GitHub open-issues read-only preview (Settings + Automations) |
| **C Gate runner** | UI presets + execute; persists gates/events; goal engine already runs real cargo/flutter checks |
| **D Packaging** | `install-with-dashboard.sh`, `--with-dashboard`, docs-site V2 API |

## Verify before you plan

```bash
ANYCODE_BUILD_DASHBOARD_UI=1 ./scripts/build-dashboard-ui.sh
cargo test -p anycode-dashboard
cd crates/dashboard-ui && npm test && npm run test:e2e
ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode --features embedded-ui
anycode dashboard --open
```

## Explicitly **not** done (pick for V3+)

| Item | Effort | Notes |
|------|--------|-------|
| **Connector OAuth + write** | L | GitHub/Linear/Slack sync, webhooks |
| **SSO / multi-user RBAC** | L | Beyond loopback auto-trust |
| **UI → agent control** | L | Start/cancel runs from Web; security model |
| **Browser gate automation** | M | Headless verify custom `done_when` browser/visual rules |
| **Tauri desktop app** | L | Wrap embedded UI |
| **Saved-hours KPI** | S–M | Heuristic from session duration × hourly rate |
| **Per-provider cost** | M | Read model catalog prices instead of env USD/M defaults |

## Suggested V3 directions

1. **Agent control plane** — read-only → approved actions (cancel run, re-run gate)
2. **Connector depth** — OAuth, issue linking to sessions, Linear read-only
3. **Cost accuracy** — provider/model price table + per-session breakdown
4. **Production deploy** — reverse proxy auth, TLS, backup automation

**Planning entry:** [`digital-workbench-next-steps.md`](digital-workbench-next-steps.md) (中文: [`digital-workbench-next-steps-zh.md`](digital-workbench-next-steps-zh.md))

## Key paths

| Path | Purpose |
|------|---------|
| `crates/dashboard/` | API, SQLite, recorder, metrics, gate_runner, connectors |
| `crates/dashboard-ui/` | React app, Playwright e2e |
| `crates/cli/src/commands/dashboard.rs` | CLI entry |
| `docs/digital-workbench-v2-complete.md` | V2 engineering checklist |
| `docs-site/guide/dashboard.md` | User guide |

## Environment reference

| Variable | Purpose |
|----------|---------|
| `ANYCODE_DASHBOARD_RECORD` | `0` disables recording |
| `ANYCODE_DASHBOARD_DB` | Override SQLite path |
| `ANYCODE_DASHBOARD_STATIC` | Override UI dist dir |
| `ANYCODE_BUILD_DASHBOARD_UI` | `1` forces UI build during `cargo build` |
| `ANYCODE_DASHBOARD_INPUT_USD_PER_M` | Token cost estimate input $/M (default 3) |
| `ANYCODE_DASHBOARD_OUTPUT_USD_PER_M` | Token cost estimate output $/M (default 15) |
| `ANYCODE_DASHBOARD_BLOCKED_ALERT_THRESHOLD` | Alert when blocked sessions exceed N |
| `GITHUB_TOKEN` / `ANYCODE_GITHUB_TOKEN` | GitHub API for connector preview |

## Decision log

- **Trust without gates:** completed sessions with zero required gates → `verified`.
- **Connectors:** GitHub issues read-only POC; other types config-only.
- **Manual gates:** UI runs use `manual_gate` session; non-required gates (do not block trust).
- **Conversation thread:** `user_prompt` + `assistant_response` events.
- **UI bundle:** release uses `rust-embed` when dist exists at compile time.
