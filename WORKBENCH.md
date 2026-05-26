# Digital Workbench (anycode dashboard)

**Status: V3 Week 1–10 complete** (2026-05) — local MVP + observability + control plane + live cancel + UI trigger run + Web tool approval + Conversations approval workflow.

## Quick start

```bash
anycode dashboard --open
# or install with embedded UI:
./scripts/install-with-dashboard.sh
```

## Ship checklist

See **[docs/digital-workbench-STATUS.md](docs/digital-workbench-STATUS.md)** (one page). Historical control-plane closure: **[docs/archive/workbench/digital-workbench-closure-report.md](docs/archive/workbench/digital-workbench-closure-report.md)**.

## Plan next (Tier 2+)

| Language | Document |
|----------|----------|
| 中文 | **[docs/digital-workbench-next-steps-zh.md](docs/digital-workbench-next-steps-zh.md)** |
| English | [docs/digital-workbench-next-steps.md](docs/digital-workbench-next-steps.md) |

Also: [STATUS](docs/digital-workbench-STATUS.md) · [control plane](docs/digital-workbench-control-plane.md) · [deploy](docs/digital-workbench-deploy-production.md) · [API](docs/digital-workbench-api.md) · [production convergence log](docs/production-convergence-log.md)

## Code

| Path | Role |
|------|------|
| `crates/dashboard/` | API, SQLite, recorder |
| `crates/dashboard-ui/` | React UI |
| `crates/cli/src/commands/dashboard.rs` | CLI |

## Tests

```bash
cargo test -p anycode-dashboard
cd crates/dashboard-ui && npm test && npm run test:e2e
```

## Not in scope (Tier 2+)

SSO/RBAC · Connector OAuth/write · Tauri
