---
title: Releases & feature flags
description: Versioning expectations, GitHub Releases, and anycode enable/disable flags.
summary: Where updates ship; how to toggle experimental runtime features from the CLI.
read_when:
  - You publish or consume anyCode builds.
  - You want a single entry point for experimental toggles.
---

# Releases & feature flags

## 0.39.0 (workspace)

- **Plan tree (session-scoped)**: plans/todos persist in SQLite; Plan panel + Build review bar (pick model, then execute).
- **Git bar**: change stats and Commit & Push above the composer.
- **Workbench icon dock**: removed top tab bar; five header icons open the side panel (Esc to close).
- **Desktop window drag**: frameless window draggable from chrome and empty areas.
- **Layout**: hide transcript when workbench is open; narrower composer; merged Pause/Send button.
- **Browser**: auto-open side panel only on new live Browser tool calls.

## 0.35.0 (workspace)

- **Memory auto-memory (automem)**: a forked sandboxed agent extracts session transcripts and consolidates them in four phases (orient→gather→consolidate→prune/index), producing a bounded `MEMORY.md` index entry (≤200 lines / ≤25KB).
- **autoDream gating**: time/session thresholds + mutex + cursor; background writes are mutex-protected.
- **Local-engine fallback**: automatically falls back to dedup/promote/forget + vector recall when no LLM is available; `memory.automem` exposes configuration switches.

## 0.3.4 (workspace)

- **Built-in skills refresh**: promote bake-off frontend/docs/design skills; drop overlapping delivery starters; move Office builders to `scripts/office/`.
- **Verification coexist**: keep `verify-discover` and ship obra `verification-before-completion`.
- **EN daily/weekly briefs**: downgrade to `internal-comms`; keep Chinese `cn-*`.

## 0.3.3 (workspace)

- **Brain + capability slots**: text-only chat can attach images via OCR; Speak uses the TTS slot; STT stays on the mic slot.
- **Bubble UX**: user messages show original text + image thumbnails; OCR stays under the hood (`model_prompt`).
- **Portal courseware demo**: county-coffee market-research 8-page FDE deck (`/demos/courseware/`).

## 0.3.2 (workspace)

- **Discoverable verification**: Discover→Search→Run (`verify-discover` skill + hollow-completion evidence nudge + `verify_recipe` memory).
- **Start shortcircuit removed**: host “site already running :8080” no longer hijacks docker/real starts.
- **Plans**: Free new users get 20M tokens; Cloud 5h stays 1,000 calls/window; Pro ¥599/mo is 10,000 calls/5h window (Pro model temporarily unavailable).
- **Portal cases**: https://anycode.work/cases/… offers openable in-browser demos.
- **Home slash modes**: Workbench home supports `/拷问` and `/目标`.
- **CI**: cross-platform `mime_to_ext`; rustfmt drift fixes.

## 0.3.1 (workspace)

- **Deliverable cards**: spreadsheet previews, HTML preview sidecars for Office/PDF, PPT slide grid, inline table cards, Mermaid blocks.
- **Viewer consolidation**: shared `selectDeliverableViewer` for chat cards and workbench file preview.
- **Skill emit**: `anycode-ppt` and office starters emit `ANYCODE_ARTIFACT` with sidecar.

## 0.3.0 (workspace)

- **Grill Me**: align before action via one `AskUserQuestion` at a time.
- **Team handoff**: LAN mDNS (ADR 015) + cloud A2A streaming relay (ADR 016, no OSS).
- **Release packaging**: macOS ships **`anyCode_<version>_aarch64.dmg` only** (CLI bundled); Linux/Windows via `cargo install` or source.

## 0.2.2 (workspace)

- **Release packaging**: macOS ships **`anyCode_<version>_aarch64.dmg` only** (CLI bundled inside the app). Standalone Linux/Windows CLI tarballs are no longer attached on tag; use `cargo install` or build from source.
- **Desktop CI**: ad-hoc codesign when no Apple Developer ID secrets; tag-triggered desktop release runs macOS only.

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
- **GitHub Releases**: tag push attaches **macOS Tauri `.dmg` only** — CLI is bundled inside `anyCode.app` at `Contents/Resources/resources/bin/anycode` (see [Digital Workbench — Desktop app](./dashboard#desktop-app-macos)). **Linux / Windows** are not published as release tarballs; install via `cargo install` or from source (`scripts/install.sh --method source`).
- **Docs** (`docs/user/` via account-portal): publish at **https://anycode.work/docs/** (`cd crates/account-portal && npm run build`).

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
