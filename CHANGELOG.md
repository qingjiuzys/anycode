# Changelog

## Unreleased

### Breaking (terminal / session / env)

- **`config.json`**: top-level key **`"tui"` → `"terminal"`** (same inner fields, e.g. `alternateScreen`). No serde alias for the old key.
- **Session directory**: default **`~/.anycode/sessions/`** (was `tui-sessions`). On first access, if `sessions` does not exist and `tui-sessions` does, the directory is **renamed** (or copied+removed on cross-device failure).
- **Environment variables** (old → new; old names are **not** read):

| Old | New |
|-----|-----|
| `ANYCODE_TUI_ALT_SCREEN` | `ANYCODE_TERM_ALT_SCREEN` |
| `ANYCODE_TUI_CLEAR_ON_START` | `ANYCODE_TERM_CLEAR_ON_START` |
| `ANYCODE_TUI_SYNC_DRAW` | `ANYCODE_TERM_SYNC_DRAW` |
| `ANYCODE_TUI_MOUSE` | `ANYCODE_TERM_MOUSE` |
| `ANYCODE_STREAM_REPL_INLINE_LEGACY` | `ANYCODE_TERM_REPL_INLINE_LEGACY` |
| `ANYCODE_STREAM_REPL_ALT_SCREEN` | `ANYCODE_TERM_REPL_ALT_SCREEN` |
| `ANYCODE_STREAM_REPL_INLINE_PCT` | `ANYCODE_TERM_REPL_INLINE_PCT` |
| `ANYCODE_STREAM_EXIT_SCROLLBACK_DUMP` | `ANYCODE_TERM_EXIT_SCROLLBACK_DUMP` |
| `ANYCODE_STREAM_SMOOTH_SCROLL` | `ANYCODE_TERM_SMOOTH_SCROLL` |

- **Rust / i18n**: internal module `crate::tui` → **`crate::term`**; Fluent bundle **`tui.ftl` → `term.ftl`**, message ids **`tui-*` → `term-*`**.

- **Stream REPL (`anycode repl` on TTY):** **UI-thread axis** — the thread running `run_stream_repl_ui_thread` (`StreamReplUiSession`) owns crossterm `poll`/`read` + ratatui `draw`, and each frame drains **pending approvals / user questions**, **finished-turn summary expiry**, and **`tick_executing_stream_transcript`** (live `build_stream_turn_plain` into `transcript` while `stream_exec_messages` is set; Tokio worker only publishes the `messages` `Arc` + anchors after spawn). Tokio `select!`, slash dispatch, and turn joins run on a **scoped sibling thread** with a **`current_thread` runtime** (`stream_repl_tokio_worker`). **Default** remains **alternate-screen fullscreen** (see `stream_repl_use_alternate_screen`); **`ANYCODE_TERM_REPL_INLINE_LEGACY=1`** restores main-buffer Inline + `insert_before` scrollback. Resize events are ignored only in Inline legacy mode; fullscreen uses ratatui autoresize.
- **Stream REPL (`anycode repl` on TTY):** after a **successful** turn, stop appending the `finish_stream_spawned_turn` transcript tail onto the message-derived `build_stream_turn_plain` rebuild — it duplicated `repl-task-ok` / output blocks and looked like “one task, multiple stacked panels”.
- **Fullscreen TUI:** fix prompt starvation when `approval_rx` / `uq_rx` stay ready — run status-line, exec/compact completion, and **`crossterm` keyboard poll** after every `tokio::select!` wake (not only the timer branch). Also ignore `KeyEventKind::Release` and filter Enter-repeat like the stream REPL (Kitty-style press/release pairs).
- **TUI / terminal palette:** default colors track claude-code-rust (`src/cli/ui.rs`): purple secondary for brand and welcome borders; **ACCENT orange** for user lines, H1 headings, menu selection, and the stdio `▸ anycode>` prompt chevron; lavender assistant labels; gray **thinking…** caption (150,150,150) beside a bold purple `✶` HUD bullet; blockquote body slightly violet-gray; muted purple-gray horizontal rules. `NO_COLOR` still forces neutral `Reset` foregrounds in ratatui paths. Markdown LRU cache keys include a palette version.
- **Copy (EN):** transcript wait line `term-germinating` is now “Thinking…” (was “Germinating…”), aligned with Claude-style wording.
- **Stream REPL (TTY Inline):** transcript 主区由「整页 dim」改为按行语义色（`Turn failed` 等错误高亮、`❯` 用户行、会话恢复/命令总览偏品牌色、斜杠帮助表灰字、正文默认白字）；`/help` 去掉与表格重复的一长行 `repl-help-cmds`，等价 `run` 示例里的 cwd 改为可读路径并加引号；`slash_commands::help_lines` 列宽按显示宽度对齐。**Dock** 与全屏 TUI 对齐：执行中/审批/选题时 **两行 HUD**（`✶` + `⎿` 提示），脚标为 ctx / provider；**底部横线在脚标之上**（prompt → rule → footer）；底栏下横线与顶横线同为 `style_horizontal_rule`。**修复**：`ReplSink::Stream` 的 `eprint_line` 不再写 stderr（避免长错误/JSON 与 Inline 视口网格交错叠字）；artifact 列表与粘贴截断提示在 Stream 下走 transcript；**每帧清空整个 Inline 视口**再绘制（避免双缓冲残留 cell 与底栏 `─` 假叠字）；主区行宽仅由 ratatui `Paragraph` 截断，避免与 `unicode-width` 二次截断错位。
- **Core:** `CoreError::CooperativeCancel` for cooperative turn/nested cancel (same `Display` as legacy `LLMError("cancelled")`); `CoreError::is_cooperative_cancel` and `anyhow_error_is_cooperative_cancel` for callers. ADR `docs/adr/002-cooperative-cancel-and-nested-agents.md`; docs-site architecture section; `TaskStop` JSON note aligned with background nested cancel wiring.
- **Tests:** `repl_line_session` helpers for trailing-assistant pop + cancel detection; MCP wall-timeout `CoreError` maps to `TimedOut` IO. **Integration:** `tests/cli_e2e_mock_llm.rs` — local TCP OpenAI-compatible mock exercises `anycode run`, `run --workflow` (two steps), and non-TTY `repl` with two or three natural-language turns (no real API key). **Stream REPL:** `repl_inline::stream_repl_keyboard_tests` — expanded `handle_event` (Ctrl+L/D、历史去重、Esc 与多行光标、BackTab、Enter 同 Tab 补全、控制字符过滤等)、dock+审批高度、`apply_stream_*_key`（y/p/n、菜单 Down、选题 Up 环绕）。

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
- **TUI:** while an agent turn is running, **Ctrl+C** requests **cooperative cancel** (same `execute_turn_from_messages` flag as nested tasks); when idle, Ctrl+C still means “press twice to quit”. Help panel (`?`) documents the distinction.
- **Stream REPL (`anycode repl` on a TTY):** **Ctrl+C** during an in-flight turn requests the same cooperative cancel (no longer treats empty prompt as EOF while the dock shows a running turn).
- **Stdio line mode** (non-TTY `repl` / bare `anycode`): while a turn is running, a **`tokio::signal::ctrl_c`** side task sets the same cooperative flag; cancelled turns print the same **`term-turn-cooperative-cancelled`** line as stream/TTY paths. A second interrupt may still kill the process depending on the OS.
- **MCP (`tools-mcp`):** optional **`ANYCODE_MCP_CALL_TIMEOUT_SECS`** wall-clock cap for a single **`tools/call`** (stdio session, rmcp, legacy SSE, and **`ANYCODE_MCP_COMMAND`** one-shot), distinct from per-line **`ANYCODE_MCP_READ_TIMEOUT_SECS`**.
