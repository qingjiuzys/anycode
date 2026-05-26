# Production convergence log

Progress for the [Production Convergence Tree](../.cursor/plans/production_convergence_tree_106434e5.plan.md) and follow-up Production Readiness Convergence audit (plan files not edited in-repo).

## Iter 1 — P0 Stabilize (2026-05-24)

| Node | Status | Notes |
|------|--------|-------|
| P0-A Docs | done | deploy checklist updated; harness cross-links; STATUS aligned |
| P0-B Control plane | done | workflow `SESSION_ENV`; approval sweep on startup/doctor; trigger sandbox copy |
| P0-C CI baseline | done | fmt/clippy/test workspace; dashboard 61+ tests; e2e 28; release build OK (2026-05-24) |

## Iter 2 — P1-A/B Trace + Budget

| Node | Status | Notes |
|------|--------|-------|
| M1 Execution trace | done | `GET /api/sessions/{id}/trace`; log_parser aligned |
| M2 Runtime budget | done | `execute_turn` budget param; degrade blocks nested/shell/MCP tools |

## Iter 3 — P1-C/D Eval + Tool governance

| Node | Status | Notes |
|------|--------|-------|
| M3 Trajectory eval | done | mock scenarios + CI `scripts/eval/run.py --with-mock` |
| M4 Tool governance | done | `anycode_tools::catalog` re-exports `anycode_core::tool_catalog` SSOT |

## Iter 4 — P1-E + P2-A MCP + Runbook

| Node | Status | Notes |
|------|--------|-------|
| M5 MCP governance | partial | strict/quota in tools; dashboard doctor MCP check |
| P2-A Runbook | done | `scripts/post-deploy-smoke.sh`; deploy doc updated |

## Iter 5 — P2-C/D/E Workflow + Memory + Ops UI

| Node | Status | Notes |
|------|--------|-------|
| M6 Workflow validation | done | validate before `run_workflow_definition` |
| M7 Memory retention | done | CLI `anycode memory prune --dry-run/--apply`; Settings data panel documents the flow |
| M8 Workbench ops UI | done | Home budget-exceeded banner; replay budget/trace phases |

## Readiness convergence audit (2026-05-24)

| Area | Status | Verified fact |
|------|--------|---------------|
| UI trigger approval path | done | Removed `-I` from dashboard-triggered `anycode run`; `sandbox_note` and API docs now describe Web approvals instead of headless bypass |
| Runtime budget | done | `execute_task` and `execute_turn` hard-stop as failure; cost budget uses token-based USD estimate; REPL/nested paths can opt in via `ANYCODE_TASK_*` env |
| Execution trace | done | `GET /api/sessions/{id}/trace` has backend unit coverage and is queried from session detail UI |
| Trajectory eval | done | Mock eval verifies required/forbidden trace events and CI runs `python3 scripts/eval/run.py --with-mock` |
| Tool/MCP governance | done | Core tool catalog is SSOT; tools crate has constant parity test; generic `mcp` calls now use the same strict/quota checks as proxied MCP tools |
| Workflow validation | done | Local executor rejects unsupported `required_gates` and `parallel_group` instead of silently ignoring them |
| Ops docs/UI | done | Deploy docs align on `projects.db`, port `43180`, smoke script, backup/restore, MCP env, budget env, and memory prune |

## Residual risks

- SSO/RBAC/Connector write — deferred Tier 2
- MCP denial/quota outcomes are surfaced as `tool_denied`; per-call MCP success detail still uses the generic `tool_call_end` envelope
- Non-loopback threat model — requires explicit remote env overrides

## Verification commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo test -p anycode-dashboard
cd crates/dashboard-ui && npm test && npm run test:e2e
python3 scripts/eval/run.py --with-mock
ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode --features embedded-ui
```
