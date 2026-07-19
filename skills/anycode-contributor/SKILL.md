---
name: anycode-contributor
description: Conventions for developing in the anyCode Rust monorepo (CLI, agent, dashboard).
description_zh: anyCode Rust 工作区开发约定（CLI、Agent、工作台）。
name_zh: anyCode 贡献者
---

# anycode-contributor

## Architecture

- **Single orchestration path**: extend `AgentRuntime` / tools / bootstrap — do not add parallel agent execution engines.
- **Composition root**: `crates/bootstrap/src/bootstrap/runtime.rs::initialize_runtime`.
- **New tools**: `crates/tools` → registry + `SECURITY_SENSITIVE_TOOL_IDS` if needed.

## Workflow

- After substantive changes: run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`.
- Default release build: `cargo build --release -p anycode-desktop-channel-bridge`.
- Dashboard UI: `crates/dashboard-ui/`; API: `crates/dashboard/`.

## Scope

- Minimize diff; match existing naming and module layout.
- Do not commit unless the user asks.
