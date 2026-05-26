# Production convergence report (2026-05-24)

**Target tier:** Local/single-machine production + Tier 1.5 Production Harness.

## Delivered

1. **Control plane hardening (P0)** — approval IPC stale sweep; stale active-session sweep; workflow session env; UI trigger now uses normal Web approvals instead of `-I`; doctor pending/MCP checks.
2. **Harness core (P1)** — execution trace API + UI query; runtime budget on `execute_task` + `execute_turn`; token/cost/duration hard stops fail the turn; budget degrade tool blocking; trajectory eval validates trace events; unified tool catalog SSOT.
3. **Ops (P2)** — post-deploy smoke script; workflow pre-run validation for unsupported execution fields; memory prune docs/UI status; Home ops summary; session replay trace phases + trace event count.

## Verification

Completed locally on 2026-05-24:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets`
- `cargo test --workspace`
- `cargo test -p anycode-dashboard`
- `cd crates/dashboard-ui && npm test -- --run && npm run test:e2e`
- `python3 scripts/eval/run.py --with-mock`
- `ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode --features embedded-ui`

See [production-convergence-log.md](production-convergence-log.md) for the detailed checklist and readiness audit notes.

## Deferred (unchanged)

SSO/RBAC · Connector OAuth/write · Tauri · Postgres · browser visual gates

## Residual risks

| Risk | Mitigation |
|------|------------|
| Stale approval/active files after CLI crash | Startup + doctor sweep (`sweep_stale_pending`, `sweep_stale_active`) |
| UI trigger approval bypass | Fixed: dashboard trigger no longer passes `-I`; Web approval inbox handles sensitive tools |
| MCP strict mode off by default | Doctor warns; set `ANYCODE_MCP_STRICT=1` and `ANYCODE_MCP_ALLOWED_TOOLS` for stricter production |
| Non-loopback binding | Token + remote override env vars required |
