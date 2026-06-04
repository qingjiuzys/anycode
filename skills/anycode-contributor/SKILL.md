---
name: anycode-contributor
description: Conventions for developing in the anyCode Rust monorepo (CLI, agent, dashboard).
---

# anycode-contributor

## Architecture

- **Single orchestration path**: extend `AgentRuntime` / tools / bootstrap — do not add parallel agent execution engines.
- **Composition root**: `crates/cli/src/bootstrap/runtime.rs::initialize_runtime`.
- **New tools**: `crates/tools` → registry + `SECURITY_SENSITIVE_TOOL_IDS` if needed.

## Workflow

- After substantive changes: run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`.
- Default release build: `cargo build --release -p anycode`.
- Dashboard UI: `crates/dashboard-ui/`; API: `crates/dashboard/`.

## Scope

- Minimize diff; match existing naming and module layout.
- Do not commit unless the user asks.
