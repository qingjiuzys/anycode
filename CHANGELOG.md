# Changelog

## 0.2.0

Workspace release: TUI-first CLI, channels, skills, nested agents with cooperative cancel (through in-flight LLM), MCP/LSP hardening and CI feature-matrix tests. Details below.

- **Default CLI entry (`anycode` with no subcommand): fullscreen TUI** on an interactive TTY (same as `anycode tui`); non-TTY stdin falls back to line-at-a-time stdio REPL. Use `anycode repl` for the Inline stream dock layout.
- TUI (no subcommand): default **DEC alternate screen** (OpenClaw-style full-viewport buffer). For main-buffer scrollback: **`export ANYCODE_TUI_ALT_SCREEN=0`** or **`ANYCODE_TUI_ALT_SCREEN=0 anycode`** (a standalone assignment line is not visible to the child); or **`"tui": { "alternateScreen": false }`** in `config.json`.
- Align Z.ai / BigModel GLM model IDs with OpenClaw; add `coding_cn` / `general_cn` plans and `open.bigmodel.cn` endpoints.
- Add Google Gemini model catalog and picker in `anycode model`; improve routing wizard (provider catalog + z.ai endpoints).
- Channel credential helpers: `anycode channel telegram-set-token`, `discord-set-token`.
- WeChat bridge: no `ApprovalCallback` for tools (headless channel parity with Telegram/Discord).
- Anthropic `chat`: retry on 429/5xx with `Retry-After` support.
- Skills: `skills.registry_url` JSON manifest (`extra_scan_roots`), `skills.agent_allowlists` for per-agent prompt sections.
- Document channel hub module and update docs-site (config-security, releases).
- **Agent** / **Task** `run_in_background: true`: spawn nested `AgentRuntime` via `tokio::spawn`; **`TaskOutput`** exposes `background_status` / `background_summary`; **`TaskStop`** on the same UUID sets a cooperative cancel flag (shared `Arc<AtomicBool>`) polled at nested **turn** and **tool** boundaries, during **`chat`**, **`chat_stream` open**, and **stream `recv`** (`tokio::select!` with ~20ms polling—no `tokio-util`), with **`AbortHandle`** still as hard fallback. HTTP bodies may still run to completion on the wire after the runtime stops awaiting; syscall-blocking tools remain best-effort. **`config.json` `lsp`** section for **`tools-lsp`** (command, `workspace_root`, `read_timeout_ms`).
- Post-P5 hardening: regression test when sub-agent depth is exhausted and **`run_in_background`** is set; **`lsp_root_uri_json`** unit tests + fake stdio **`lsp_forward_shell`** tests (`cargo test -p anycode-tools --features tools-lsp` in CI); roadmap / issue draft / config-security updates for **v2** cooperative cancel and channel **AskUserQuestion**.
- **MCP stdio (`tools-mcp`):** optional **`ANYCODE_MCP_READ_TIMEOUT_SECS`** for JSON-RPC line reads; clearer timeout and unexpected-EOF errors (incl. child exit status when known); **`McpStdioSession::stdio_child_is_running`** for health checks. Roadmap tables: **v2 cancel** moved to **Recently shipped**; **MCP beyond stdio** marked partial. CI runs **`cargo test -p anycode-tools --features tools-mcp`**. Integration tests: **`mcp_tools_call_shell`** + **`McpStdioSession::connect`** when the child exits before any JSON-RPC reply (EOF / broken pipe).
