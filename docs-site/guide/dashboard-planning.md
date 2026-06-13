---
title: Digital Workbench — Planning
description: V1–V3 completion status and 0.3 web console roadmap.
---

# Digital Workbench — Planning

**Status:** V1 MVP + V2 + V3 control plane are **complete** for local single-user use (2026-05).

**Next (0.3):** **Web account console** — login, plan/subscription, usage, billing, API keys, enterprise admin. **Agent execution stays in the terminal**; the web UI is not an Agent operator in 0.3.

Use this page when deciding what to build next. Full detail lives in the repo under `docs/`.

## What's done

| Layer | Delivered |
|-------|-------------|
| CLI | `anycode dashboard`, doctor, status, token, db backup |
| Data | SQLite recording from run/goal/workflow/repl/cron |
| Trust | Gates → blocked; gate-less completed → verified |
| UI | React/Vite, zh/en, SSE, 12+ pages, embedded release UI |
| Auth (local) | `/login`, session API, loopback `local_trusted` |
| V2–V3 | Tokens, connectors, gate runner, local control plane, e2e |

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
| [roadmap.md §3.5](https://github.com/qingjiuzys/anycode/blob/main/docs/roadmap.md) | **0.3 SSOT** — web console packages |
| [digital-workbench-next-steps.md](https://github.com/qingjiuzys/anycode/blob/main/docs/workbench/digital-workbench-next-steps.md) | Planning detail (EN) |
| [digital-workbench-api.md](https://github.com/qingjiuzys/anycode/blob/main/docs/workbench/digital-workbench-api.md) | API contract |
| [production-harness-hardening.md](https://github.com/qingjiuzys/anycode/blob/main/docs/planning/production-harness-hardening.md) | **0.4** runtime hardening (not 0.3) |

User guide: [Digital Workbench](./dashboard.md).

## 0.3 — web console (not built as product shell)

| Area | Target |
|------|--------|
| Account | Login, user menu, account settings |
| Plan | Subscription tier, upgrade CTAs (mock OK) |
| Usage | Token/cost dashboard for end users |
| Billing | Invoices shell (no real payment in 0.3) |
| API | Key create/revoke/rotate |
| Enterprise | Org, members, roles, audit entry |

**Out of scope for 0.3:** operating Agent from the browser as a product promise; cloud Agent hosting; real payment gateways.

## Planning questions

1. **Sidebar IA?** Plan · Usage · Billing · API · Account · Enterprise
2. **Entitlement model?** Free / Pro / Team; quota vs seats
3. **Remote auth?** API token only vs email/password session
4. **Agent on web?** Default **no** for 0.3 — use CLI

See [next-steps](https://github.com/qingjiuzys/anycode/blob/main/docs/workbench/digital-workbench-next-steps.md).
