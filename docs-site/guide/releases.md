---
title: Releases & feature flags
description: Versioning expectations, GitHub Releases, and anycode enable/disable flags.
summary: Where updates ship; how to toggle experimental runtime features from the CLI.
read_when:
  - You publish or consume anyCode builds.
  - You want a single entry point for experimental toggles.
---

# Releases & feature flags

## 0.2.0 (workspace)

- **Models**: Z.ai / 智谱 GLM catalog aligned with OpenClaw `model-definitions` ids; `plan` values `coding_cn` / `general_cn` map to `open.bigmodel.cn` endpoints; Google Gemini picker catalog; `anycode model` routing wizard uses the OpenClaw provider list + z.ai plan menu.
- **Channels**: `telegram-set-token` / `discord-set-token` subcommands; `anycode_channels::hub` documents the single `ChannelMessage` → `build_channel_task` flow; WeChat bridge no longer registers an interactive tool-approval callback.
- **LLM**: Anthropic non-stream `chat` retries on 429/5xx with `Retry-After` (same policy shape as the z.ai client).
- **Skills**: optional `skills.registry_url` manifest merge, `skills.agent_allowlists` for per-agent prompt sections, `SkillCatalog::render_prompt_subsection_allowlist`.
- **Agent**: nested **`run_in_background`** with cooperative cancel through tool boundaries and in-flight **`chat` / stream** (**`TaskStop`** on the nested task id).
- **Sessions (TUI & stream REPL)**: on the main **`execute_turn_from_messages`** path, **Ctrl+C** while a turn is running requests the same cooperative cancel flag (fullscreen TUI: first Ctrl+C cancels the turn; second Ctrl+C when idle still means quit; TTY **`anycode repl`**: Ctrl+C cancels an in-flight turn instead of exiting on an empty prompt).
- **MCP / LSP**: MCP stdio **`ANYCODE_MCP_READ_TIMEOUT_SECS`** (per-line JSON-RPC read), optional **`ANYCODE_MCP_CALL_TIMEOUT_SECS`** (whole **`tools/call`** round-trip); clearer timeout/EOF errors, **`McpStdioSession::stdio_child_is_running`**; **`config.json` `lsp`**; CI **`tools-lsp`** / **`tools-mcp`** test jobs.

## Versioning

- **Library / CLI version** follows the workspace `version` in the root `Cargo.toml`.
- **GitHub Releases**: tag and attach `anycode` binaries for your platform when distributing outside `cargo install`.
- **Docs site** (VitePress under `docs-site/`): deploy to GitHub Pages with `VITEPRESS_BASE=/your-repo/` when using project pages.

## Runtime feature flags {#runtime-feature-flags}

Use the CLI as the **single toggle surface**:

```bash
anycode enable skills
anycode disable workflows
anycode status
```

Recognized names (see `anycode_core::FeatureFlag`):

| Flag | `enable` / `disable` name |
|------|---------------------------|
| Skills scanning in CLI | `skills` |
| Workflow helpers | `workflows` or `workflow` |
| Goal-oriented mode affordances | `goal-mode` or `goal` |
| Channel-oriented defaults | `channel-mode` or `channel` |
| Experimental approval path | `approval-v2` or `approval` |
| Context compaction affordances | `context-compression` or `compact` |
| Workspace profile overlays | `workspace-profiles` or `workspace` |

## Related

- [CLI overview](./cli) — global flags  
- [Routing](./routing) — `model_routes` and workspace overlays  
