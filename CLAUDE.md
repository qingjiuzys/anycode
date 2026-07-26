# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

**Build and test:**
```bash
# Standard development workflow
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo build --release -p anycode-channel-bridge   # anycode-daemon binary
./scripts/sync-desktop-dev.sh              # UI-only (~15s)
./scripts/sync-desktop-dev.sh --rust       # UI + Rust (release-local ~1–2min)
# Shipping DMG: ./scripts/build-desktop-local.sh (LTO release)

# Feature-specific testing
cargo test -p anycode-tools --features tools-lsp
cargo test -p anycode-tools --features tools-mcp

# Docs site preview (account-portal)
cd crates/account-portal && npm install && npm run dev
```

**Before committing:**
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets`
- `cargo test --workspace`

## Workspace Architecture

**anyCode** is a Desktop + daemon AI workbench (Rust workspace, Tokio). `AgentRuntime` is the sole authority for multi-turn LLM+tool execution.

### Crate Structure

- **`apps/anycode-desktop`** — Tauri shell; embeds dashboard in-process
- **`crates/channel-bridge`** — `anycode-daemon` (channels + scheduler)
- **`crates/config`** — `config.json` schema
- **`crates/bootstrap`** — `initialize_runtime` composition root
- **`crates/dashboard`** — Workbench HTTP API + embedded UI
- **`crates/dashboard-ipc`** — approval/question/cancel file IPC
- **`crates/agent`** — `AgentRuntime`, tool loop, compaction
- **`crates/core`** — domain types and traits
- **`crates/llm`** — LLM providers
- **`crates/tools`** — built-in tools and registry
- **`crates/security`** — approval and policy
- **`crates/memory`** — memory backends
- **`crates/channels`** — channel abstractions
- **`crates/locale`** — Fluent i18n

### Key Patterns

**Orchestration (ADR 000):** Only `execute_task` and `execute_turn_from_messages` orchestrate multi-turn loops.

**Composition root (ADR 002, superseded path):** `crates/bootstrap/src/runtime.rs::initialize_runtime` — used by Desktop, dashboard embedded chat, and `anycode-daemon`.

**Cooperative cancel (ADR 010):** `Arc<AtomicBool>` at turn/tool boundaries.

### Product Entry Points

1. **anyCode.app** — Workbench at `http://127.0.0.1:43180` (in-process dashboard)
2. **`anycode-daemon`** — `scheduler` (built-in cron)
3. **Dev** — `cargo tauri dev` in `apps/anycode-desktop`

The terminal `anycode` CLI (REPL/TUI/`run`/`setup`) is **removed**.
Third-party IM channel bridges (WeChat / Telegram / Discord) are **removed**; conversations happen in the local Workbench only.

### Configuration

- `~/.anycode/config.json` — `crates/config`
- Workbench `/setup` for first-time model configuration
- `ANYCODE_IGNORE_APPROVAL`, `ANYCODE_DASHBOARD_*`, etc.

### Cron Scheduler

`crates/channel-bridge` hosts the built-in cron scheduler (`anycode-daemon scheduler`); jobs persist in `~/.anycode/tasks/orchestration.json`, results land in the Workbench session and `~/.anycode/logs/cron-runs.jsonl`.

### Common Tasks

**Add a tool:** `crates/tools/src/` → `registry.rs` → `SECURITY_SENSITIVE_TOOL_IDS` → bootstrap policy.

**Add LLM provider:** `crates/llm/src/providers/` + `transport_for_provider_id`.

### Documentation

- User: `docs/user/` (published at https://anycode.work/docs/ via account-portal)
- Maintainer: `docs/architecture.md`, `docs/ops/run-flow.md`
- ADRs: `docs/adr/`
