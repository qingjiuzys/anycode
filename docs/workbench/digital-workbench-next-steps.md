# Digital Workbench — Next Steps (planning)

**You are here:** V1 MVP + V2 + **V3 Week 1–10 (control plane) complete** for local single-user use. See [archive/workbench/digital-workbench-closure-report.md](../archive/workbench/digital-workbench-closure-report.md) for the frozen scope and verification summary.

**Next recommended slice:** [Production Harness Hardening](../planning/production-harness-hardening.md) — a Tier 1.5 pass that adds execution trace, runtime budgets, trajectory eval, tool/MCP governance, declarative workflow validation, and memory retention before larger Tier 2/3 expansion.

## Current capability (done)

| Area | What works today |
|------|------------------|
| **CLI** | `anycode dashboard` (+ doctor, status, token, db backup) |
| **Recording** | run / goal / workflow / repl / cron → SQLite + SSE |
| **Trust** | Gates block delivery; gate-less completed → verified |
| **UI** | 12+ pages, zh/en, lazy routes, embedded release UI |
| **Observability** | Global + per-project tokens, CSV export, timeline, readiness |
| **Alerts** | `blocked_threshold_exceeded` when blocked sessions > N |
| **Connectors** | GitHub + Linear open-issues read-only |
| **Gate runner** | UI presets → shell + SSE streaming + run history |
| **Control plane** | Live cancel, UI trigger run/goal, Web tool approval, Conversations workflow |
| **Goal verify** | Engine runs real cargo/flutter checks; `[gate]` lines ingested |
| **Install** | `./scripts/install-with-dashboard.sh` |
| **Tests** | 59 Rust dashboard tests + 28 Playwright e2e |

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
| [`archive/workbench/digital-workbench-closure-report.md`](../archive/workbench/digital-workbench-closure-report.md) | **Control-plane closure summary (start here)** |
| [`digital-workbench-STATUS.md`](digital-workbench-STATUS.md) | One-page ship checklist |
| [`digital-workbench-control-plane.md`](digital-workbench-control-plane.md) | Cancel / trigger / approval behavior |
| [`archive/workbench/digital-workbench-handoff.md`](../archive/workbench/digital-workbench-handoff.md) | Full handoff + deferred backlog |
| [`digital-workbench-api.md`](digital-workbench-api.md) | API contract |
| [`production-harness-hardening.md`](../planning/production-harness-hardening.md) | Tier 1.5 runtime hardening roadmap |

中文：[digital-workbench-next-steps-zh.md](digital-workbench-next-steps-zh.md)

---

## Tier 1.5 — Production Harness Hardening

Do this before connector write-back, multi-user auth, or desktop packaging. The slice keeps `AgentRuntime` as the single orchestration authority and turns the completed Workbench control plane into a production-grade harness.

| Priority | Item | Effort | Outcome |
|----------|------|--------|---------|
| P0 | **Execution trace SSOT** | L | Structured task/turn/LLM/tool/gate/budget events power replay, eval, audit, and provenance |
| P0 | **Runtime budget** | L | Token/cost/duration budgets warn, degrade, or hard-stop during execution |
| P0 | **Trajectory eval** | M | CI catches repeated tools, forbidden tools, failed gates, and budget violations even when final text looks OK |
| P1 | **Tool governance metadata** | M | Tool catalog records risk, category, approval policy, agent visibility, and audit level |
| P1 | **MCP governance** | M | Optional strict whitelist, per-server quotas, and MCP trace events |
| P1 | **Declarative workflow validation** | M–L | Planner emits plans; the harness validates agents, tools, gates, budgets, and dependencies before execution |
| P2 | **Memory retention** | M | Hot/vector memory supports dry-run prune, retention scoring, and evidence provenance |
| P2 | **Workbench operations UI** | M | Dashboard explains budget health, trace replay, trajectory verdicts, tool risk, and memory retention |

Recommended order: trace first, then runtime budget, then trajectory eval. Those three create the foundation for every later hardening item.

## Deferred (Tier 2–3)

Do **not** expand the V3 control-plane slice further without a new planning pass.

| Item | Effort | Why deferred |
|------|--------|--------------|
| **Connector OAuth / write** | L | Needs OAuth + write threat model |
| **Pending-approval notifications** | M | Policy design + delivery channels |
| **SSO / OIDC** | L | Required before non-loopback multi-user |
| **RBAC enforcement** | L | Wire roles in permissions doc |
| **Browser gate automation** | M–L | Headless visual rules beyond shell checks |
| **Tauri desktop** | L | Wrap embedded UI; offline dist |

---

## Decision prompts for next sprint

1. **Audience next?** Solo dev (stay local) vs small team (auth/RBAC) vs CI integration (API tokens + export only)
2. **Primary metric?** Cost, trust/gates, or throughput (sessions/day)
3. **Connector value?** GitHub/Linear read-only enough, or write-back/Slack blocking?
4. **Control appetite?** Observation-only vs approved actions from Web (control plane slice is **done** for local loopback)

Answer these → pick deferred items → open issues from [`archive/workbench/digital-workbench-handoff.md`](../archive/workbench/digital-workbench-handoff.md).
