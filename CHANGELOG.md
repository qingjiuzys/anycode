# Changelog

## 0.2.0

- TUI (no subcommand): default **DEC alternate screen** (OpenClaw-style full-viewport buffer). For main-buffer scrollback: **`export ANYCODE_TUI_ALT_SCREEN=0`** or **`ANYCODE_TUI_ALT_SCREEN=0 anycode`** (a standalone assignment line is not visible to the child); or **`"tui": { "alternateScreen": false }`** in `config.json`.
- Align Z.ai / BigModel GLM model IDs with OpenClaw; add `coding_cn` / `general_cn` plans and `open.bigmodel.cn` endpoints.
- Add Google Gemini model catalog and picker in `anycode model`; improve routing wizard (provider catalog + z.ai endpoints).
- Channel credential helpers: `anycode channel telegram-set-token`, `discord-set-token`.
- WeChat bridge: no `ApprovalCallback` for tools (headless channel parity with Telegram/Discord).
- Anthropic `chat`: retry on 429/5xx with `Retry-After` support.
- Skills: `skills.registry_url` JSON manifest (`extra_scan_roots`), `skills.agent_allowlists` for per-agent prompt sections.
- Document channel hub module and update docs-site (config-security, releases).
