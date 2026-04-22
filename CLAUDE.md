# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

**Build and test:**
```bash
# Standard development workflow
cargo fmt --all -- --check       # Format check (CI requires this)
cargo clippy --workspace --all-targets  # Lint check (CI requires this)
cargo test --workspace           # Run all tests
cargo build --release -p anycode # Build release binary

# Feature-specific testing
cargo test -p anycode-tools --features tools-lsp
cargo test -p anycode-tools --features tools-mcp

# Single test execution
cargo test --package anycode --test cli_smoke -- help_prints_usage
cargo test --package anycode-tools --test skills_catalog -- test_skills_manifest

# Docs site preview
cd docs-site && npm install && npm run dev
```

**Before committing:**
Always run CI-equivalent checks to avoid remote failures:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets`
- `cargo test --workspace`
- If docs changed: `cd docs-site && npm ci && npm run build`

## Workspace Architecture

**anyCode** is a terminal-first AI assistant built as a Rust workspace with Tokio async runtime and ratatui-based stream UI. The architecture follows a strict orchestration pattern where `AgentRuntime` is the sole authority for multi-turn LLM+tool execution.

### Crate Structure

- **`crates/cli`** - Main binary (`anycode`), stream terminal + transcript shared layer (`term`), REPL, config, channel bridges (WeChat, Telegram, Discord)
- **`crates/agent`** - `AgentRuntime`, tool loop, compaction, memory integration, agent implementations
- **`crates/core`** - Domain types and traits: Task, Message, Tool, Agent, SecurityPolicy, MemoryPipeline, LLM types
- **`crates/llm`** - LLM provider abstractions and implementations (Anthropic, OpenAI-compatible, Bedrock, etc.)
- **`crates/tools`** - Built-in tools (Bash, Edit, Glob, Grep, MCP, LSP, etc.) and tool registry
- **`crates/security`** - Security layer with allow/deny rules and approval callbacks
- **`crates/memory`** - Memory store and embedding pipeline implementations
- **`crates/channels`** - Channel abstractions for WeChat, Telegram, Discord
- **`crates/locale`** - i18n support with Fluent bundles

### Key Architectural Patterns

**Orchestration Authority (ADR 000):**
- `AgentRuntime::execute_task` and `execute_turn_from_messages` are the ONLY multi-turn orchestration paths
- The `Agent` trait supplies type, tool subset, and system prompt hooks but `Agent::execute` is NOT the main CLI path
- Never create parallel execution engines - extend Tool + registry + bootstrap instead

**Composition Root (ADR 002):**
- `crates/cli/src/bootstrap/runtime.rs::initialize_runtime` assembles the runtime: LLM client, tool registry, security layer, memory backends
- All CLI entry points (interactive terminal, REPL, `run`) share this single runtime construction path

**Cooperative Cancel (ADR 002):**
- Use `Arc<AtomicBool>` for cooperative cancellation at turn/tool boundaries
- `CoreError::CooperativeCancel` for cancellation results (same display as legacy "cancelled")
- `anyhow_error_is_cooperative_cancel` helper for handling `anyhow::Error`

**Tool Extension Points:**
- Default tools: `crates/tools/src/registry.rs` + `catalog.rs` + `SECURITY_SENSITIVE_TOOL_IDS` (must stay aligned)
- LLM providers: `crates/llm/src/providers/` + `transport_for_provider_id`
- Approval/deny: `crates/security` + SecurityLayer callbacks from bootstrap

### Runtime Modes

The CLI supports three main modes:
1. **Stream terminal** (default on TTY): ratatui stream UI (alternate screen by default; see `stream_repl_use_alternate_screen`)
2. **Stream REPL** (`anycode repl`): same stack with explicit subcommand; optional Inline legacy via env
3. **Line REPL** (non-TTY): stdio line-at-a-time mode

### Terminal / transcript layer (`term`)

The shared layer (`crates/cli/src/term/`) provides styles, input, session snapshots, approval plumbing, and transcript building for the stream REPL. Key areas:
- **`terminal_guard`**: alternate-screen / inline policy (`ANYCODE_TERM_*` stream REPL variables)
- **`transcript/`**: plain-text and tool render helpers for the dock
- **`session_persist`**: `~/.anycode/sessions` JSON snapshots

### Configuration

- Main config: `~/.anycode/config.json` (schema in `crates/cli/src/app_config_schema.rs`)
- Session-level overlays via `.anycode/config.json` in workspace directories
- Environment variables: `ANYCODE_IGNORE_APPROVAL`, `ANYCODE_TERM_ALT_SCREEN`, `ANYCODE_TERM_REPL_*`, etc. (see `CHANGELOG.md` for renames)

### Testing Patterns

- **Unit tests**: Inline in source files with `#[cfg(test)]`
- **Integration tests**: `crates/*/tests/` directories
- **E2E tests**: `crates/cli/tests/` with mock LLM server for realistic scenarios
- **Feature flags**: Test with specific features: `--features tools-mcp,tools-lsp`

### Channel Bridges

Channel implementations in `crates/cli/src/`:
- **WeChat**: `wechat_*.rs` files with iLink扫码 support
- **Telegram**: `tg.rs` with token setup helpers
- **Discord**: `discord_channel.rs` with token setup helpers

All channels use the same `AgentRuntime` via `initialize_runtime` but with different approval callbacks (headless for channels, interactive for CLI).

### Important Development Notes

1. **No new public traits** until two real implementations need them (prefer enums, free functions, or `pub(crate)`)
2. **No plugin/dynamic loading** - extend through Tool registry and LLM providers
3. **MCP/LSP tools** are feature-gated: `tools-mcp`, `tools-lsp`, `mcp-oauth`
4. **Cursor rules** (`.cursor/rules/`) emphasize: clear structure, self-check, testing, release builds, CI alignment
5. **Memory**: File-based store with optional vector backends (Sled, OpenAI-compatible embeddings)

### Common Tasks

**Add a new tool:**
1. Implement `Tool` trait in `crates/tools/src/`
2. Register in `build_registry_with_services` (`registry.rs`)
3. Add to `SECURITY_SENSITIVE_TOOL_IDS` if needed
4. Update policy registration in `bootstrap/runtime.rs`

**Add a new LLM provider:**
1. Add transport variant in `crates/llm/src/lib.rs`
2. Implement provider module in `crates/llm/src/providers/`
3. Wire up in `transport_for_provider_id` and provider catalog

**Stream / terminal layer modifications:**
- Transcript: `term/transcript/`
- REPL event/draw: `repl/stream_*.rs`, `repl/dock_render.rs`

### Documentation

- **User docs**: `docs-site/` (VitePress, bilingual)
- **Architecture**: `docs/architecture.md`, `docs-site/guide/architecture.md`
- **ADRs**: `docs/adr/` (design decisions)
- **Roadmap**: `docs/roadmap.md` (maintainer backlog)
