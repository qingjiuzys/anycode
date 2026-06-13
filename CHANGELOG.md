# Changelog

## Unreleased

## 0.2.2

### Changed

- **Release packaging**: macOS GitHub Release ships **`.dmg` only`** with CLI bundled in `anyCode.app`; Linux/Windows keep standalone CLI tarballs/zips.
- **Desktop CI**: ad-hoc codesign when Apple Developer ID secrets are absent; tag-triggered `desktop-release` runs **macOS only** (Windows/Linux desktop jobs are manual `workflow_dispatch`).
- **Build scripts**: cross-platform icon venv in `build-dashboard-ui.sh`; Linux desktop builds skip `media-local` unless on macOS.

### Fixed

- **Desktop release CI**: Windows dashboard UI build no longer fails on `bin/pip`; macOS no longer fails codesign with empty identity.

## 0.2.1

### Added

- **Docs / README:** Product positioning for BYOK, personal WeChat bridge, local Workbench, automations, macOS native STT/OCR, and enterprise-friendly integration boundaries. Model validation scope documents maintainer checks on **GLM** and **DeepSeek** vs catalog-supported providers.
- **Docs site:** Updated home features, getting-started experience paths, docs directory index, and DeepSeek configuration section.
- **WeChat bridge:** Outbound media CDN delivery, setup QR SVG, and `SendWeChatMessage` tool for agent-initiated file sends.
- **macOS desktop:** Apple Speech STT and Apple Vision OCR via `anyCode.app` native media stack.

- **Digital Workbench:** V3 control plane closed at `v3_week10` (live cancel, UI trigger run, Web tool approval, Conversations workflow). Closure report: [docs/archive/workbench/digital-workbench-closure-report.md](docs/archive/workbench/digital-workbench-closure-report.md). Playwright e2e 28 tests.
- **Setup / config wizard:** Memory step is **five choices** (`Skip`, `noop`, **Markdown/`hybrid`**, **pipeline + HTTP** embeddings with optional base-URL probe, optional **local ONNX** when `--features embedding-local`). Default menu highlight targets **Hybrid** Markdown preset; pure `file`, pipeline without vectors remain **JSON-only**. **`anycode config`** reuses the same step; non‑TTY prints `setup-memory-non-tty-hint`.
- **WeChat bridge:** Align inbound body with OpenClaw `bodyFromItemList`; slash routing uses first plain TEXT segment; `ref_msg` quote lines (including title-only quotes) and media selection/fallback (`IMAGE > VIDEO > FILE > VOICE` without STT); CDN decryption for video/file/voice when applicable; attachment prompts via `wx.ftl`.
- **`anycode setup`:** Fourth interactive choice and `--channel skip` / `--channel none` to skip channel onboarding. Non-interactive setups without `--channel` skip channel; pass `--channel wechat|telegram|discord` explicitly when needed.
- **Channel cron:** `workspace-assistant` exposes `CronCreate` / `CronDelete` (plus `CronList`). Built-in `anycode scheduler` uses `~/.anycode/tasks/scheduler.lock` for single-instance scheduling; **WeChat**, **Telegram**, and **Discord** bridges spawn the same scheduler task so cron can fire without a separate `anycode scheduler` when a bridge holds the lock.
- **WeChat cron delivery:** `CronCreate` wall-clock jobs store weekday `*` (avoids ISO vs Sun=1 weekday mismatch). Scheduler pushes the reminder to the last WeChat chat **before** running the agent task; optional second message when the agent returns a long reply.

- **`tools-mcp`:** `McpStdioSession::call_tool_named` short-circuits when the stdio child has already exited (**`mcp_stdio_dead`** in JSON); reconnect policy stays **manual** — see [`docs/adr/007-mcp-session-reconnect-policy.md`](docs/adr/007-mcp-session-reconnect-policy.md).
- **Cron observability:** builtin scheduler appends `~/.anycode/logs/cron-runs.jsonl` (`started` / `ok` / `error`); see [`docs/ops/cron-observability.md`](docs/ops/cron-observability.md).
- **`CronCreate` IANA `schedule_timezone`:** `Asia/Shanghai`-style names convert wall-clock fields to UTC storage (in addition to `local` / `utc` / `utc0` / `gmt`).
- **Agent stream failover:** `pop_assistant_placeholder` removes streaming assistant placeholders before non-stream chat fallback.
- **Providers:** OpenClaw-style aliases (`doubao`→`volcengine`, `modelstudio`→`alibaba`, `gemini`→`google`, `open-router`, `nim`→`nvidia`, `ernie`→`qianfan`, `chatgpt`→`openai`, `zhipu`/`zhipu-ai`→`z.ai`, `deepseek-ai`, `x-ai`, `byte-plus`, `claude-cli`/`anthropic-cli`→`anthropic`, `azure-openai`→`openai`, `venice-ai`→`venice`, `stepfun-ai`/`chutes-ai`/`sglang-ai`, `opencode-ai`/`synthetic-ai`, `litellm-ai`/`kilocode-ai`, `deepseek-chat`, `byteplus-ai`, `moonshot-v1`, `together-api`, `amazon-bedrock-api`, `vllm-api`, `groq-cloud`, `openai-api`, `custom-api`, `moonshot-api`, etc.).
- **Cron:** `schedule_timezone` accepts `Zulu` as a UTC alias.
- **WeChat bridge:** outbound `send_text` retries transient HTTP errors with capped backoff; reply chunk failures are logged after retries.

### Fixed

- **Stream REPL:** run `tick_executing_stream_transcript` after `draw_stream_frame` and sync viewport width on alternate-screen resize so executing turns do not duplicate transcript lines when the terminal is resized.
- **Agent runtime:** stream→chat fallback pops the streaming assistant placeholder before appending the final message (no duplicate assistant rows on model failover).
- **OpenAI-compatible tools:** collapse nullable `anyOf` / `oneOf` branches in tool parameter schemas before requests (DeepSeek and similar gateways).
- **Memory pipeline:** log `tracing::warn` when embedding or vector upsert/search fails; keyword and hot-store recall continue.
- **`CronCreate`:** reject invalid cron expressions with field-count and parse hints; unknown `schedule_timezone` values return a clear error.
- **`WebFetch`:** block literal private, loopback, link-local, CGNAT `100.64.0.0/10`, and documentation hosts (including IPv6 `::1`, IPv4-mapped loopback, decimal and hexadecimal IPv4 hostnames, `*.localhost`) before fetch; cap redirects and strip URL credentials on each hop; resolve hostnames and reject DNS answers that map to private/link-local IPs (redirect hops included).
- **Providers:** fix `zhipu-ai` kebab alias mapping to `z.ai` (CI `normalize_openclaw_aliases`).
- **cli_smoke:** line REPL test uses isolated noop-memory config so parallel WeChat bridge runs do not lock `memory.sled`.

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
- **Core:** `CoreError::CooperativeCancel` for cooperative turn/nested cancel (same `Display` as legacy `LLMError("cancelled")`); `CoreError::is_cooperative_cancel` and `anyhow_error_is_cooperative_cancel` for callers. ADR `docs/adr/010-cooperative-cancel-and-nested-agents.md`; docs-site architecture section; `TaskStop` JSON note aligned with background nested cancel wiring.
- **Tests:** `repl_line_session` helpers for trailing-assistant pop + cancel detection; MCP wall-timeout `CoreError` maps to `TimedOut` IO. **Integration:** `tests/cli_e2e_mock_llm.rs` — local TCP OpenAI-compatible mock exercises `anycode run`, `run --workflow` (two steps), and non-TTY `repl` with two or three natural-language turns (no real API key). **Stream REPL:** `repl_inline::stream_repl_keyboard_tests` — expanded `handle_event` (Ctrl+L/D、历史去重、Esc 与多行光标、BackTab、Enter 同 Tab 补全、控制字符过滤等)、dock+审批高度、`apply_stream_*_key`（y/p/n、菜单 Down、选题 Up 环绕）。

### Changed

- **Digital Workbench UI:** Home suggestions, modal overlay, settings navigation, glass-skin styling, and assistant transcript cleanup.

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
