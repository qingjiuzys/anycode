# ADR 007: MCP stdio session health and reconnect (policy)

Status: **Accepted** (policy) — **2026-04-20**  
Implementation: **no automatic stdio reconnect** + **fail-fast** on dead transport (see below). Optional config **`mcp.auto_restart`** (global or per server) remains a **future** hook; default stays **off** until reconnect is implemented end-to-end.

## Context

- Long-lived MCP connections use [`McpStdioSession`](../../crates/tools/src/mcp_session.rs).
- Operational rules today: line read timeouts, optional `tools/call` wall timeouts, **`stdio_child_is_running`**, **no automatic reconnect** ([`docs/mcp-stdio-lifecycle.md`](../mcp-stdio-lifecycle.md), roadmap §6).
- Users still see opaque failures when the child has exited but the session object remains in a registry.

## Decision (current product behavior)

1. **No silent auto-reconnect** for stdio MCP: restarting a server must remain an explicit user or bootstrap action to avoid surprise command re-exec, duplicate init, or auth side effects.
2. **Fail fast on dead transport**: `call_tool_named` checks **`stdio_child_is_running`** first and returns a **`ToolOutput`** with `mcp_stdio_dead: true` and a clear error string instead of hanging on a broken pipe.
3. **Future controlled reconnect** (if implemented) must: (a) live behind an explicit config flag or per-server policy, (b) re-run `initialize` / `tools/list`, (c) update the tool registry atomically, and (d) be documented in this ADR as **Accepted** with a migration note in `CHANGELOG.md`.

## Consequences

- Registry layers may **short-circuit** dead sessions without new traits.
- Full **reconnect** remains out of scope until the checklist above is implemented and reviewed.
