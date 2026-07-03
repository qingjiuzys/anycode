# Digital Workbench

**Status: V3 Week 1–10 complete** (2026-05) — local MVP + observability + control plane + live cancel + UI trigger run + Web tool approval + Conversations approval workflow.

## Quick start

**macOS:** launch **anyCode.app** (Workbench opens at `http://127.0.0.1:43180`).

**Developers:** `cargo tauri dev` in `apps/anycode-desktop`, or run the embedded dashboard from a dev build.

## Ship checklist

See **[docs/workbench/digital-workbench-STATUS.md](docs/workbench/digital-workbench-STATUS.md)**.

## Plan next (Tier 2+)

| Language | Document |
|----------|----------|
| 中文 | **[docs/workbench/digital-workbench-next-steps-zh.md](docs/workbench/digital-workbench-next-steps-zh.md)** |
| English | [docs/workbench/digital-workbench-next-steps.md](docs/workbench/digital-workbench-next-steps.md) |

Also: [STATUS](docs/workbench/digital-workbench-STATUS.md) · [control plane](docs/workbench/digital-workbench-control-plane.md) · [deploy](docs/workbench/digital-workbench-deploy-production.md) · [API](docs/workbench/digital-workbench-api.md)

## Code

| Path | Role |
|------|------|
| `crates/dashboard/` | API, SQLite, recorder |
| `crates/dashboard-ui/` | React UI |
| `apps/anycode-desktop/` | Tauri shell + in-process dashboard |

## Tests

```bash
cargo test -p anycode-dashboard
cd crates/dashboard-ui && npm test && npm run test:e2e
```

## Not in scope (Tier 2+)

SSO/RBAC · Connector OAuth/write
