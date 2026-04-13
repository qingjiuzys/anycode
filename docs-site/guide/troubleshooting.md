---
title: Troubleshooting
description: Fix common anyCode issues by symptom, with clear next actions.
summary: Start from what failed, apply quick checks, then move to advanced diagnosis only if needed.
read_when:
  - Setup, channel binding, or command execution failed.
  - You need a quick “what to do next” list.
---

# Troubleshooting

For users who need fast recovery steps when something fails.

How to use this page:

1. Find the symptom closest to your error.
2. Run the listed quick checks in order.
3. Move to advanced diagnostics only if quick checks fail.

## 1) `anycode` command not found

1. Run `anycode --help`.
2. If not found, check PATH notes from installer output.
3. Open a new terminal and retry.
4. If still failing, reinstall from [Install](./install).

## 2) `setup` cannot ask questions / freezes

- Run in a real terminal (not CI/headless session).
- For SSH/CI environments, prepare config first or run setup locally.
- If you only need one channel, use explicit choice:

```bash
anycode setup --channel wechat
anycode setup --channel telegram
anycode setup --channel discord
```

Expected output: setup goes directly to the selected channel path.

## 3) WeChat QR login fails

- Bind on a machine with browser/GUI.
- Run:

```bash
anycode channel wechat
```

Expected output: QR flow appears and asks for confirmation/binding steps.

- If task folder is wrong after binding, set project path via `/cwd` in WeChat.

## 4) API request failed

- Re-run `setup` and confirm provider/model/api key.
- Check endpoint matches provider protocol (OpenAI-compatible vs provider-native API).
- For Google provider, prefer setup defaults and avoid custom non-compatible endpoint paths.

## 5) Approval prompts block your workflow

- `require_approval=true` means sensitive tools need confirmation.
- If you know the risk and need one-time bypass:

```bash
anycode run --ignore-approval --agent general-purpose "..."
```

Expected output: task runs without interactive approval prompts in this process.

## Advanced diagnostics (optional)

- **MCP / `McpAuth` / OAuth (no GUI):** anycode does not open a browser for you. Use the dynamic **`mcp__…__authenticate`** tool or **`McpAuth`**, read **stderr** from the MCP subprocess (same terminal as the CLI), then complete OAuth in your browser. See [Config & security — MCP OAuth](./config-security#mcp-oauth-mcpauth-no-gui) and env **`ANYCODE_MCP_READ_TIMEOUT_SECS`** / **`ANYCODE_MCP_CALL_TIMEOUT_SECS`** if calls hang.
- Developer logs/tests: see [Development](./development)

## Still stuck

- Open an issue with:
  - OS version
  - `anycode --version`
  - redacted `~/.anycode/config.json` (remove API keys)
