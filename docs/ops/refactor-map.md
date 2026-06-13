# Refactor Map

Maintainer guide for moving code without changing behavior.

## Ownership Boundaries

- `crates/core`: stable domain types and cross-crate contracts.
- `crates/agent`: runtime orchestration (`AgentRuntime` sole authority).
- `crates/tools`: tool implementations.
- `crates/dashboard`: Digital Workbench backend.
- `crates/cli`: binary composition root, dispatch, channels, terminal UX.
- `crates/dashboard-ui`: embedded React UI; shared UI in `src/components/ui`.
- `docs` / `docs-site` / `scripts`: as before.

## Completed (2026-05)

| Area | Result |
| --- | --- |
| Dashboard API / DB | `handlers/*`, `db/store/{open,*} + facades` |
| Agent runtime | `budget.rs`, `memory_hooks.rs`, `execute_{turn,task,goal,tool}.rs`, `tool_result_injection.rs`, `nested_task.rs`; `mod.rs` ~375 lines |
| Agent tests | `tests/{support,unit,integration}.rs` |
| CLI args | `cli_args/*` |
| CLI dispatch | `commands/dispatch/{mod,channel_cmds,ops,task_cmds}.rs`; `main.rs` ~42 lines |
| CLI channels | `channels/*`; `super::` / `crate::channels::` call paths |
| CLI config | `app_config/{schema/{types,validation},prompts,model_interactive/{mod,provider_flows,routing},...}` |
| CLI workbench / workflow | `workbench/*`, `tasks/workflow_exec.rs` |
| Core | `tool_catalog.rs` |
| Dashboard UI | `api/{http,client/*,types/*}`, `routes/{lazyPages,routes}.tsx`, `hooks/useRuntimeSettings.ts`, `pages/settings/*` (all sections) |
| Docs / CI | ADR 010, workbench archive, release `embedded-ui` |

## Verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
cd crates/dashboard-ui && npm run build && npm run test && npm run test:e2e
cargo build --release -p anycode
```
