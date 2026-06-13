# anyCode

Terminal-first, self-hosted **BYOK** AI assistant: ask questions and run tasks from the CLI, bridge the same runtime to **personal WeChat**, Telegram, or Discord, or open the local **Digital Workbench** for projects, sessions, and scheduled jobs.

**Languages:** [简体中文](README.zh.md)

- **Docs site:** <https://qingjiuzys.github.io/anycode/>
- **CLI binary:** `anycode`
- **License:** [MIT](LICENSE)

## What makes anyCode different

- **Single Rust runtime** — one `AgentRuntime` orchestrates LLM + tools (Bash, Edit, Grep, MCP, LSP, Skills, Cron, Knowledge, and more). Execution stays on your machine; not a cloud-hosted agent gateway.
- **Personal WeChat bridge** — scan QR to bind iLink; send tasks from your phone, approve sensitive tools in chat, and receive files/images back. See [WeChat guide](docs-site/guide/wechat.md).
- **Local Digital Workbench** — `anycode dashboard --open` for projects, sessions, assets, automations, security approvals, and REST API integration. See [Workbench tour](docs-site/guide/workbench.md).
- **Automations** — natural-language cron jobs with run history, guardrails, and optional WeChat notifications. See [Scheduled jobs](docs-site/guide/cli-scheduler.md).
- **BYOK model catalog** — 30+ providers aligned with OpenClaw (z.ai/GLM, DeepSeek, Anthropic, Bedrock, Copilot, OpenRouter, Ollama, custom endpoints, and more). See [Models & endpoints](docs-site/guide/models.md).
- **Enterprise-friendly integration** — local Workbench REST API, API tokens, project policies, eval/gate harness, and documented permission modes for secondary development. SSO/RBAC is on the roadmap, not production-ready yet.
- **macOS-first desktop experience** — the **anyCode.app** Tauri shell bundles the Workbench and adds **Apple Speech** (native STT, no Whisper download) and **Apple Vision OCR** (on-device text extraction). Browser-only Workbench at `127.0.0.1` does not include these native features.

## Model validation scope

anyCode integrates many LLM providers, but **maintainer day-to-day validation** focuses on:

- **z.ai / GLM** (default chat stack, e.g. `glm-5`)
- **DeepSeek** (OpenAI-compatible API, including tool-schema normalization)

**CI** exercises the agent loop against a **local mock OpenAI-compatible server** — not live vendor APIs.

All other catalog providers are **configuration-supported**. After you add credentials, verify with `anycode status`, the Workbench model probe, or a short test chat. See [Models & endpoints](docs-site/guide/models.md) for details.

## Quick start (3 steps)

1. Install anyCode
2. Run `anycode setup` to configure the model and optional channels
3. Run a task to verify

**macOS (recommended):** download **`anyCode_<version>_aarch64.dmg`** from [Releases](https://github.com/qingjiuzys/anycode/releases), open it, and drag **anyCode** to Applications. The desktop app **bundles the CLI** (`anycode` sidecar inside the app) and opens Workbench automatically — no separate CLI tarball on macOS.

**Linux:**

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

**Windows PowerShell:**

```powershell
irm https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.ps1 | iex
```

**macOS headless / developers:** use `install.sh` if you want a standalone CLI on PATH without the desktop app:

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

**After install:**

```bash
anycode --help
anycode setup
anycode run --agent general-purpose "Reply with OK only"
```

If you see `command not found`, check PATH notes in the install guide.

**Open the Workbench (optional):**

```bash
anycode dashboard --open
```

**macOS desktop app:** primary macOS deliverable — `.dmg` with bundled CLI, Workbench, Apple Speech STT, and Apple Vision OCR. Terminal CLI from the app bundle:

```bash
/Applications/anyCode.app/Contents/Resources/resources/bin/anycode --help
```

## Documentation

- [Getting started](docs-site/guide/getting-started.md)
- [Install](docs-site/guide/install.md)
- [Models & endpoints](docs-site/guide/models.md)
- [Digital Workbench](docs-site/guide/workbench.md)
- [WeChat & setup](docs-site/guide/wechat.md)
- [Scheduled jobs](docs-site/guide/cli-scheduler.md)
- [Troubleshooting](docs-site/guide/troubleshooting.md)
- [Full docs directory](docs-site/guide/docs-directory.md)

**Chinese (仓库内 Markdown):** [快速开始](docs-site/zh/guide/getting-started.md) · [安装](docs-site/zh/guide/install.md) · [模型与端点](docs-site/zh/guide/models.md) · [工作台导览](docs-site/zh/guide/workbench.md) · [微信与 setup](docs-site/zh/guide/wechat.md) · [排错](docs-site/zh/guide/troubleshooting.md) · [文档地图](docs-site/zh/guide/docs-directory.md)

## For developers

**Implementation stack:** Rust workspace (`cargo`); async runtime **Tokio**; terminal UI **ratatui** + **crossterm**; Markdown **pulldown-cmark**; i18n **Fluent** (`fluent-bundle`); code highlighting **syntect**. Runtime is split across crates such as `anycode-core`, `anycode-agent`, `anycode-llm`, and `anycode-tools` (MCP/LSP).

```bash
cargo fmt
cargo clippy
cargo test --workspace
cargo build --release -p anycode
```

Preview the docs site locally:

```bash
cd docs-site && npm install && npm run dev
```
