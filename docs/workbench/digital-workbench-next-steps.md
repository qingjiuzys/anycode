# Digital Workbench — Next Steps (planning)

**You are here:** V1 MVP + V2 + **V3 Week 1–10 (local control plane) complete**. See [archive/workbench/digital-workbench-closure-report.md](../archive/workbench/digital-workbench-closure-report.md) for the frozen scope summary.

**Next recommended slice (0.3):** **Web account console** — login, subscription/billing shell, usage & entitlements, API keys, and enterprise admin entry points. Agent **execution stays in CLI / local runtime**; the web UI is **not** an Agent operator in 0.3. See [`roadmap.md`](../roadmap.md) §3.5.

## Current capability (done)

| Area | What works today |
|------|------------------|
| **CLI** | `anycode dashboard` (+ doctor, status, token, db backup) |
| **Recording** | run / goal / workflow / repl / cron → SQLite + SSE |
| **Trust** | Gates block delivery; gate-less completed → verified |
| **UI** | 12+ pages, zh/en, lazy routes, embedded release UI |
| **Auth (local)** | `/login`, `/api/auth/*`, loopback `local_trusted` |
| **Observability** | Global + per-project tokens, CSV export, timeline, readiness |
| **Alerts** | `blocked_threshold_exceeded` when blocked sessions > N |
| **Connectors** | GitHub + Linear open-issues read-only |
| **Gate runner** | UI presets → shell + SSE streaming + run history |
| **Control plane (local)** | Live cancel, UI trigger run/goal, Web tool approval, Conversations workflow |
| **Goal verify** | Engine runs real cargo/flutter checks; `[gate]` lines ingested |
| **Install** | `./scripts/install-with-dashboard.sh` |
| **Tests** | 69+ Rust dashboard tests + 28 Playwright e2e |

**Verify anytime:**

```bash
ANYCODE_BUILD_DASHBOARD_UI=1 ./scripts/build-dashboard-ui.sh
cargo test -p anycode-dashboard
cd crates/dashboard-ui && npm test && npm run test:e2e
ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode --features embedded-ui
anycode dashboard --open
```

## Document map

| Doc | Use when |
|-----|----------|
| [`../roadmap.md`](../roadmap.md) | **0.3 SSOT** — web console packages A–E |
| [`digital-workbench-STATUS.md`](digital-workbench-STATUS.md) | One-page ship checklist |
| [`digital-workbench-api.md`](digital-workbench-api.md) | API contract (auth modes) |
| [`digital-workbench-permissions.md`](digital-workbench-permissions.md) | Roles + enterprise mode |
| [`production-harness-hardening.md`](../planning/production-harness-hardening.md) | **0.4** runtime hardening (not 0.3) |
| [`archive/workbench/digital-workbench-closure-report.md`](../archive/workbench/digital-workbench-closure-report.md) | V3 control-plane closure |

中文：[digital-workbench-next-steps-zh.md](digital-workbench-next-steps-zh.md)

---

## 0.3 — Web account console

Product shell modeled after a SaaS admin (plan, usage, billing, API, account). **Mock/local data OK** for subscription until payment integration.

| Priority | Item | Effort | Outcome |
|----------|------|--------|---------|
| P0 | **Web login & session** | M | Email/password or token login; user menu; Settings → Auth; keep `local_trusted` on loopback |
| P0 | **Plan / subscription shell** | M | Plan tier display, subscribe/upgrade CTAs, subscription status (mock OK) |
| P0 | **Usage management** | M | Wrap existing token/cost metrics into a user-facing usage page; quota hints |
| P1 | **Billing & invoices shell** | M | Invoice list, download placeholder, billing profile (no real payment in 0.3) |
| P1 | **API management** | M | Create/revoke/rotate API keys; show last-used and scopes |
| P1 | **Enterprise admin shell** | L | Org, members, roles, audit log entry; SSO/OIDC **design only** |

Recommended order: **0.3-A → B+C → D → E** (matches [`roadmap.md`](../roadmap.md) §3.5.1).

### 0.3 out of scope

- **Operating Agent from the web** (trigger run, approve tools, cancel sessions as a product promise).
- Remote job queue, cloud-hosted Agent runtime, OpenClaw Gateway-style relay.
- Real payment gateway (Stripe, WeChat Pay, etc.).

Local V3 control-plane features remain for **loopback dev**; they are not expanded as 0.3 product scope.

---

## 0.4 — Production Harness Hardening (deferred)

Trace, runtime budget, trajectory eval, tool/MCP governance — see [`production-harness-hardening.md`](../planning/production-harness-hardening.md) and [`closure-plan-2026-06.md`](../planning/closure-plan-2026-06.md). Epic mapping in [`roadmap.md`](../roadmap.md) §4.

---

## Later (post 0.3)

| Item | Effort | Notes |
|------|--------|-------|
| **Connector OAuth / write** | L | OAuth + write threat model |
| **Full SSO / OIDC** | L | Beyond 0.3 design placeholder |
| **RBAC enforcement** | L | Wire roles in permissions doc |
| **Browser gate automation** | M–L | Headless visual rules |
| **Real billing integration** | L | After subscription shell |

---

## Decision prompts for next sprint

1. **0.3 nav IA?** Plan · Usage · Billing · API · Account · Enterprise (CODEBUDDY-style sidebar).
2. **Entitlement model?** Free / Pro / Team tiers; token quota vs seat-based.
3. **Auth for remote bind?** Token-only vs email/password + session cookie.
4. **Keep Agent off the web?** Default **yes** for 0.3 — CLI remains the execution surface.

Answer these → open issues from §3.5 packages in [`roadmap.md`](../roadmap.md).
