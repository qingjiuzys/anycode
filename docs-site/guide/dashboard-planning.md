---
title: Digital Workbench — Planning
description: V1+V2 completion status and V3 roadmap entry point.
---

# Digital Workbench — Planning

**Status:** V1 MVP + V2 slices A–D are **complete** for local single-user use (2026-05).

Use this page when deciding what to build next. Full detail lives in the repo under `docs/`.

## What's done

| Layer | Delivered |
|-------|-------------|
| CLI | `anycode dashboard`, doctor, status, token, db backup |
| Data | SQLite recording from run/goal/workflow/repl/cron |
| Trust | Gates → blocked; gate-less completed → verified |
| UI | React/Vite, zh/en, SSE, 12+ pages, embedded release UI |
| V2-A | Per-project tokens, CSV export, blocked-threshold alert |
| V2-B | GitHub open-issues read-only (Settings + Automations) |
| V2-C | Gate runner (presets, execute, DB persistence) |
| V2-D | `install-with-dashboard.sh`, docs, 11 Playwright e2e tests |

## Verify

```bash
ANYCODE_BUILD_DASHBOARD_UI=1 ./scripts/build-dashboard-ui.sh
cargo test -p anycode-dashboard
cd crates/dashboard-ui && npm test && npm run test:e2e
ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode --features embedded-ui
anycode dashboard --open
```

## Repo docs (maintainers)

| Document | Purpose |
|----------|---------|
| [digital-workbench-next-steps.md](https://github.com/qingjiuzys/anycode/blob/main/docs/workbench/digital-workbench-next-steps.md) | **Start here** — V3 tiers + sample 4-week plan |
| [digital-workbench-handoff.md](https://github.com/qingjiuzys/anycode/blob/main/docs/archive/workbench/digital-workbench-handoff.md) | Full handoff |
| [digital-workbench-v2-complete.md](https://github.com/qingjiuzys/anycode/blob/main/docs/archive/workbench/digital-workbench-v2-complete.md) | V2 checklist |
| [digital-workbench-v1-mvp.md](https://github.com/qingjiuzys/anycode/blob/main/docs/archive/workbench/digital-workbench-v1-mvp.md) | V1 UX acceptance |
| [digital-workbench-api.md](https://github.com/qingjiuzys/anycode/blob/main/docs/workbench/digital-workbench-api.md) | API contract |

User guide: [Digital Workbench](./dashboard.md).

## V3 — not built (pick your sprint)

### Tier 1 (local, high value)

- Per-provider/model cost table
- Saved-hours KPI on Home
- Gate run history + streaming output
- Linear connector read-only
- Production deploy checklist (nginx, TLS, token)

### Tier 2 (control plane — needs security design)

- Cancel running session from UI
- Trigger `anycode run` from Web
- Approval inbox for pending tools

### Tier 3 (multi-user / external)

- SSO / OIDC, RBAC
- Connector OAuth + write-back
- Browser gate automation, Tauri shell

## Planning questions

1. **Audience?** Solo dev vs team vs CI read-only integration
2. **Primary metric?** Cost vs trust/gates vs throughput
3. **Connectors?** GitHub read-only enough, or Linear/Slack blocking?
4. **Control?** Observation-only vs approved actions from Web

Answer these → open issues from the Tier 1 list in [next-steps](https://github.com/qingjiuzys/anycode/blob/main/docs/workbench/digital-workbench-next-steps.md).
