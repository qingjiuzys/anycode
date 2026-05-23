---
title: Digital Workbench
description: Local project-centric dashboard for sessions, gates, cron, and artifacts.
---

# Digital Workbench

The **Digital Workbench** is a local web UI for anyCode projects: sessions (`run`, `goal`, `workflow`, `repl`, `cron`), timeline events, trust gates, artifacts, skills, and cron ledger data. Data is stored in `~/.anycode/projects.db` and updated while you use the CLI.

## Quick start

```bash
# Build static UI (once per UI change or before release)
ANYCODE_BUILD_DASHBOARD_UI=1 ./scripts/build-dashboard-ui.sh

# Release binary with embedded UI (recommended)
cargo build --release -p anycode --features embedded-ui
anycode dashboard --open
```

If you skip the UI build, release still runs the API; `anycode dashboard doctor` warns when UI is missing.

Development with hot reload:

```bash
anycode dashboard          # API on :43180
cd crates/dashboard-ui && npm run dev   # Vite proxies /api to the server
```

## Recording

By default, `run`, `goal`, workflow steps, stream REPL, and cron scheduler runs append events to the dashboard DB. Disable with:

```bash
export ANYCODE_DASHBOARD_RECORD=0
```

Point a running server at another instance for live SSE refresh:

```bash
export ANYCODE_DASHBOARD_URL=http://127.0.0.1:43180
```

## Main surfaces

| Page | What you see |
|------|----------------|
| **Overview** | Project counts, running sessions, **token usage & est. cost**, insight cards, SSE timeline |
| **Projects** | Per-project sessions, event filter, gates, reindex, project health |
| **Conversations** | Session list + read-only thread (`user_prompt` / `assistant_response` / tools) |
| **Sessions** | Goal run inspection, trust completeness, per-gate bar, timeline, artifacts |
| **Reports** | Project/session Markdown export, copy, download |
| **Audit** | Low-risk dashboard actions (reindex, report, skills rescan) |
| **Automations** | Cron runs/jobs from `~/.anycode` ledger files |
| **Agents / Skills** | Agent role cards + scanned `SKILL.md` usage stats |
| **Assets** | FileWrite/Edit/Notebook paths; export CSV |
| **Settings** | Tabbed panels: auth, DB, service bind/port preferences, **editable LLM/fallback**, skills, **asset strict mode**, security, notifications, doctor |

Sidebar shows **workspace card**, **nav badge counts**, and **topbar search**. Overview includes **insight cards** (automation health, risks, suggestions) and live **SSE** status.

**Login:** On non-loopback binds, unauthenticated users are redirected to `/login`. Loopback (`127.0.0.1`) auto-trusts `local@anycode`.

**Preferences:** Settings → Service & port lets you save host, port, and DB path to `~/.anycode/dashboard_preferences.json`. Restart with the shown command (or `anycode dashboard` — CLI reads saved prefs; explicit `--host` / `--port` / `--db` override).

**Release build** (from repo root):

```bash
ANYCODE_BUILD_DASHBOARD_UI=1 ./scripts/build-dashboard-ui.sh
cargo build --release -p anycode --features embedded-ui
```

Release embeds UI via `embedded-ui` when `dist/` exists at compile time. Dev fallback: serve `crates/dashboard-ui/dist` from disk.

**Connectors (V2 POC):** GitHub connectors with `repo` in config show read-only open issues on Settings. Token from config or `GITHUB_TOKEN` / `ANYCODE_GITHUB_TOKEN`.

**Gate runner (V2):** Project detail → run `cargo test/clippy/fmt`, `npm test`, `playwright`, or `flutter test` presets against the project root.

**Install with embedded UI:**

```bash
./scripts/install-with-dashboard.sh   # source install + UI build + embedded-ui
# or
./scripts/install.sh --method source --source-dir "$PWD" --with-dashboard
```

**Tests:** `cd crates/dashboard-ui && npm test && npm run test:e2e` (Playwright UX smoke).

**Next planning:** [`digital-workbench-next-steps.md`](../../docs/digital-workbench-next-steps.md) · Also: [Workbench planning (V3)](./dashboard-planning.md)

### API highlights

| Endpoint | Purpose |
|----------|---------|
| `GET /api/events/{event_id}` | Single event detail |
| `GET /api/sessions?status=&trusted_status=&kind=` | Filtered session list |
| `GET /api/projects/{id}/stats` | Event/gate/session aggregates |
| `GET /api/artifacts?unverified_only=&blocked_session_only=` | Trust-filtered assets |
| `GET /api/projects/{id}/report` | Project Markdown/JSON report |
| `GET /api/sessions/{id}/report` | Session Markdown/JSON report |
| `GET /api/audit/events` | Dashboard audit log |
| `GET /api/settings/policies` | Local security policy summary |
| `GET /api/settings/data-health` | DB/project data health checks |
| `GET /api/metrics/readiness` | Delivery readiness summary |
| `GET /api/settings/service-status` | Live service / SSE / dist status |
| `GET /api/settings/doctor` | Doctor diagnostics |
| `GET /api/settings/runtime` | LLM config summary + auth/SSE paths |
| `GET/PUT /api/settings/preferences` | Dashboard bind/db + asset strict mode |
| `PUT /api/settings/llm` | Patch `~/.anycode/config.json` LLM + model fallback |
| `GET /api/metrics/timeline?days=7` | Rolling session/event timeline |
| `GET /api/metrics/usage?days=7` | LLM token totals + estimated USD cost |
| `GET /api/metrics/usage/export?days=7` | CSV export (optional `project_id`) |
| `GET /api/projects/{id}/usage?days=7` | Per-project token usage |
| `GET /api/projects/{id}/gates/presets` | Detected test/lint presets for project root |
| `POST /api/projects/{id}/gates/execute` | Run preset (`preset_id`) or custom command |
| `GET /api/settings/connectors/{id}/github/issues` | Read-only GitHub open issues |
| `GET /api/notifications/recent` | Notification feed (audit-derived) |
| `GET /api/settings/connectors` | Connector config (read-only in UI) |
| `DELETE /api/settings/notifications/{id}` | Remove notification policy |
| `PATCH /api/settings/notifications/{id}/enabled` | Enable/disable policy |
| `POST /api/skills/{id}/all-projects` | Enable/disable skill on all projects |
| `GET /api/auth/me` | Current user (loopback auto-login) |

When all required gates pass, session file artifacts are marked **verified** and linked to the verifying gate.

## CLI helpers

```bash
anycode dashboard doctor          # DB / port / dist / loopback checks
anycode dashboard status          # Quick status
anycode dashboard token create    # API token for non-loopback
anycode dashboard db check        # Migrations, table sizes, growth warnings
anycode dashboard db backup       # Copy projects.db
```

## Reports

Open **Reports** from the sidebar, or use **Generate report** on a project/session detail page (deep-links via `?project_id=` / `?session_id=`).

Reports include summary, trust status, gates, artifacts, failures, and reproduction hints. Copy Markdown or download `.md` locally — nothing is written to your repo.

## Audit

After **Rebuild index**, **Skills rescan**, or **Generate report**, events appear on the **Audit** page. V1 actor is always `local`.

## Data health

**Settings → Data health** shows DB size, orphan refs, missing project roots, stale sessions, and gate/trust mismatches. **Overview** and **Project detail** show compact warnings when checks fail.

## Reindex a project

On a project detail page, **Rebuild index** ingests historical task logs under the workspace and rescans skills. Use this after enabling the dashboard on an existing repo.

## Environment

| Variable | Purpose |
|----------|---------|
| `ANYCODE_DASHBOARD_RECORD` | `0` disables CLI recording (default: on) |
| `ANYCODE_DASHBOARD_URL` | Notify URL for SSE push after writes |
| `ANYCODE_DASHBOARD_STATIC` | Override bundled UI directory |
| `ANYCODE_BUILD_DASHBOARD_UI` | `1` — build UI during `cargo build` (release) |
| `ANYCODE_DASHBOARD_INPUT_USD_PER_M` | Input token $/M for cost estimate (default 3) |
| `ANYCODE_DASHBOARD_OUTPUT_USD_PER_M` | Output token $/M for cost estimate (default 15) |
| `ANYCODE_DASHBOARD_BLOCKED_ALERT_THRESHOLD` | Emit `blocked_threshold_exceeded` when blocked sessions exceed N (default 0) |
| `GITHUB_TOKEN` / `ANYCODE_GITHUB_TOKEN` | GitHub API token for connector issue preview |

CLI flags: `anycode dashboard --host`, `--port`, `--db`, `--static-dir`, `--open`. Saved preferences apply when flags are omitted.
