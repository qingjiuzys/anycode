---
title: Troubleshooting
description: Common anyCode issues — no TTY, WeChat QR, MCP, and documentation links.
summary: Quick fixes for non-interactive environments, bridge login, and MCP/OAuth limits.
read_when:
  - Something failed during setup, wechat, or config.
  - You run anyCode in CI or over SSH without a real TTY.
---

# Troubleshooting

## No TTY / SSH / CI

- **`anycode config`** and **`setup`** may skip interactive prompts when stdin is not a TTY. Run them **in a real terminal**, or prepare **`~/.anycode/config.json`** in advance.
- **WeChat bind** requires a graphical environment to complete QR login. If you used **`setup --skip-wechat`**, run **`anycode wechat`** later from a machine with a display.
- **`run`** / **`repl`** approval prompts need a connected terminal for stdin when `require_approval` is true. Use **`--ignore-approval`** (or `ANYCODE_IGNORE_APPROVAL`) only when you understand the risk.

## WeChat bridge

- If the wizard says to run **`anycode wechat`**, do that in an environment where the bridge can open a browser or show a QR code.
- **Working directory** for WeChat tasks defaults to **`~/.anycode/workspace`** when unset; use `/cwd` in WeChat to point at a project (see [WeChat & setup](./wechat)).

## MCP and OAuth

- MCP support requires building with **`--features tools-mcp`**. Check **`ANYCODE_MCP_COMMAND`** / **`ANYCODE_MCP_SERVERS`** and logs under **`~/.anycode/tasks/<id>/output.log`**.
- **OAuth / `McpAuth`** may require stdio interaction with the MCP server; headless or sandboxed runners might not complete OAuth. Prefer servers that support non-interactive tokens where possible.
- If tools disappear from the model, verify **`security.mcp_tool_deny_patterns`** and **`mcp_tool_deny_rules`** in [Config & security](./config-security).

## z.ai / OpenAI-compatible: no tool calls

- Some models return text only on the first turn. Try **`ANYCODE_ZAI_TOOL_CHOICE_FIRST_TURN=1`** or **`zai_tool_choice_first_turn: true`** in config (see [CLI sessions](./cli-sessions)).

## Broken or “dead” links in docs

- This site sets **`ignoreDeadLinks`** for paths into the repo **`crates/`** tree (outside the VitePress root). Those links are intentional for GitHub browsing.
- If **`vitepress build`** reports a dead link, fix the path or add an exception in **`.vitepress/config.ts`**.

## Still stuck

- Open an issue with **OS**, **anycode --version** (if available), and a redacted **`config.json`** (no API keys).  
- See [Development](./development) for running tests locally.
