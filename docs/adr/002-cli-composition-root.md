# ADR 002: CLI composition root boundaries

## Status

Accepted

## Context

The `anycode` CLI crate aggregates configuration, TUI/stream REPL, `run`, channel bridges, and supporting subcommands. A clear boundary prevents business rules from leaking into `anycode_core` and keeps `AgentRuntime` construction centralized. (Historical HTTP daemon wiring was removed; see [ADR 003](003-http-daemon-deprecated.md).)

## Decision

- **`crates/cli/src/bootstrap/runtime.rs`** (`initialize_runtime`) is the **primary composition root** for the shared `AgentRuntime` used by TUI, `run`, and long-lived bridges (e.g. WeChat) that need a shared runtime. Prompt augmentation for system context lives in `bootstrap/prompt_runtime.rs`.
- **`anycode_core`** holds domain types and traits only; it must not depend on CLI or read `config.json` directly.
- **Security policy registration** for default tools uses `catalog::SECURITY_SENSITIVE_TOOL_IDS` and `bootstrap` to align `SecurityLayer` with the tool registry—do not maintain a second list only in CLI.
- Channel bridges and optional crates (e.g. `channels`) integrate at the CLI or adapter layer, not inside `AgentRuntime` internals.

## Consequences

- New global wiring (extra `Arc` services, MCP attach) should land under `bootstrap/` or a dedicated `cli` submodule, not in `core`.
- Documentation for contributors references `bootstrap/runtime.rs` and the docs site page `docs-site/guide/contributing-extensions.md` (built as **Contributing extensions** / **扩展与贡献清单**).
