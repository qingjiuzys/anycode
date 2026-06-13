# Digital Workbench — Status (one page)

**Last updated:** 2026-05-24 · **Phase:** V3 Week 1–10 done · **Production convergence:** Local/single-machine production ready, Tier 1.5 harness closed for default scope ([log](../archive/sprints/production-convergence-log.md))

**Repo root:** [WORKBENCH.md](../WORKBENCH.md) · **Closure archive:** [archive/workbench/digital-workbench-closure-report.md](../archive/workbench/digital-workbench-closure-report.md)

## Ship checklist

| Phase | Status |
|-------|--------|
| V1 MVP (7 UX items) | ✅ |
| V2-A Observability | ✅ |
| V2-B GitHub connector POC | ✅ |
| V2-C Gate runner | ✅ |
| V2-D Packaging / install | ✅ |
| V3-W1 Per-model cost + saved-hours KPI | ✅ |
| V3-W2 Gate run history + Linear connector | ✅ |
| V3-W3 Connector doctor + session cancel + gate required | ✅ |
| V3-W4 Session usage + gate SSE streaming | ✅ |
| V3-W5 Security activity log + session token chart | ✅ |
| V3-W6 Live CLI cooperative cancel (file IPC) | ✅ |
| V3-W7 UI trigger run (sandboxed subprocess) | ✅ |
| V3-W8 Interactive Web tool approval (file IPC) | ✅ |
| V3-W9 Session-scoped inbox + pending badges + resolved log | ✅ |
| V3-W10 Conversations approval workflow (filter + inline inbox) | ✅ |
| Production deploy checklist | ✅ |
| Rust tests (`anycode-dashboard`) | ✅ 69+ (unit + integration) |
| Playwright e2e | ✅ 28 |
| Release `embedded-ui` | ✅ |
| CI dashboard job | ✅ |

## Quick start

```bash
./scripts/install-with-dashboard.sh   # or existing anycode binary
anycode dashboard --open
```

## Planning docs (read in order)

1. **[digital-workbench-next-steps.md](digital-workbench-next-steps.md)** ← **0.3** web console planning  
2. [`../roadmap.md`](../roadmap.md) §3.5 ← SSOT for 0.3 packages  
3. [digital-workbench-api.md](digital-workbench-api.md) · [digital-workbench-permissions.md](digital-workbench-permissions.md)  
4. [production-harness-hardening.md](../planning/production-harness-hardening.md) ← **0.4** runtime hardening (not 0.3)  
5. [archive/workbench/digital-workbench-closure-report.md](../archive/workbench/digital-workbench-closure-report.md)  

中文：[digital-workbench-next-steps-zh.md](digital-workbench-next-steps-zh.md)

User docs: [docs-site/guide/dashboard.md](../docs-site/guide/dashboard.md) · [Planning page](../docs-site/guide/dashboard-planning.md)

## Explicitly not in 0.3 (product)

- **Web as Agent operator** (product promise to run/approve/cancel Agent from browser)
- Real payment gateway · full IdP · Connector OAuth/write · browser visual gates

**In 0.3 scope:** login, plan/subscription shell, usage, billing shell, API keys, enterprise admin entry. See [`../roadmap.md`](../roadmap.md) §3.5.

macOS Tauri v0.1 (sidecar) shipped. Local V3 control-plane APIs remain for loopback dev only.

**Session index (2026-05-24):** SQLite is the session/conversation SSOT (`sessions.task_id` unique); stack trace is read from `output.log` on demand. Project scan no longer bulk-imports logs into sessions.

Nothing in this list blocks **using** the workbench locally today.
