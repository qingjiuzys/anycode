# anyCode

Self-hosted **BYOK** AI assistant with a local **Digital Workbench**: chat and run tasks in the browser or **anyCode.app** (macOS), bridge the same runtime to **personal WeChat**, Telegram, or Discord via **`anycode-daemon`**, and manage projects, sessions, and scheduled jobs.

**Languages:** [简体中文](README.zh.md)

- **Docs site:** <https://qingjiuzys.github.io/anycode/>
- **Headless binary:** `anycode-daemon`
- **License:** [MIT](LICENSE)

## What makes anyCode different

- **Single Rust runtime** — one `AgentRuntime` orchestrates LLM + tools (Bash, Edit, Grep, MCP, LSP, Skills, Cron, Knowledge, and more). Execution stays on your machine; not a cloud-hosted agent gateway.
- **Personal WeChat bridge** — scan QR to bind iLink; send tasks from your phone, approve sensitive tools in chat, and receive files/images back. See [WeChat guide](docs-site/guide/wechat.md).
- **Local Digital Workbench** — embedded in **anyCode.app** or dev builds at `http://127.0.0.1:43180` for projects, sessions, assets, automations, security approvals, and REST API integration. See [Workbench tour](docs-site/guide/workbench.md).
- **Automations** — natural-language cron jobs with run history, guardrails, and optional WeChat notifications. See [Scheduled jobs](docs-site/guide/cli-scheduler.md).
- **BYOK model catalog** — 30+ providers aligned with OpenClaw (z.ai/GLM, DeepSeek, Anthropic, Bedrock, Copilot, OpenRouter, Ollama, custom endpoints, and more). See [Models & endpoints](docs-site/guide/models.md).
- **Enterprise-friendly integration** — local Workbench REST API, API tokens, project policies, eval/gate harness, and documented permission modes for secondary development. SSO/RBAC is on the roadmap, not production-ready yet.
- **macOS-first desktop experience** — the **anyCode.app** Tauri shell bundles the Workbench and adds **Apple Speech** (native STT, no Whisper download) and **Apple Vision OCR** (on-device text extraction). Browser-only Workbench at `127.0.0.1` does not include these native features.

## Model validation scope

anyCode integrates many LLM providers, but **maintainer day-to-day validation** focuses on:

- **z.ai / GLM** (default chat stack, e.g. `glm-5`)
- **DeepSeek** (OpenAI-compatible API, including tool-schema normalization)

**CI** exercises the agent loop against a **local mock OpenAI-compatible server** — not live vendor APIs.

All other catalog providers are **configuration-supported**. After you add credentials, verify with the Workbench model probe or a short test chat. See [Models & endpoints](docs-site/guide/models.md) for details.

## Quick start (3 steps)

1. Install **anyCode.app** (macOS) or **`anycode-daemon`** (Linux/Windows headless)
2. Open Workbench **`/setup`** to configure the model and optional channels
3. Send a test message in the Workbench composer

**macOS (recommended):** download **`anyCode_<version>_aarch64.dmg`** from [Releases](https://github.com/qingjiuzys/anycode/releases), open it, and drag **anyCode** to Applications. The desktop app embeds the Workbench automatically.

**Linux / Windows (headless):**

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

```powershell
irm https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.ps1 | iex
```

Then open `http://127.0.0.1:43180/setup` after starting the desktop app or embedded dashboard from a dev build.

**Verify:** send “Reply with OK only” in a Workbench conversation.

## Documentation

- [Getting started](docs-site/guide/getting-started.md)
- [Install](docs-site/guide/install.md)
- [Desktop app (macOS)](docs-site/guide/desktop.md)
- [Headless daemon](docs-site/guide/daemon.md)
- [Models & endpoints](docs-site/guide/models.md)
- [Digital Workbench](docs-site/guide/workbench.md)
- [WeChat & setup](docs-site/guide/wechat.md)
- [Scheduled jobs](docs-site/guide/cli-scheduler.md)
- [Troubleshooting](docs-site/guide/troubleshooting.md)

**Chinese:** [快速开始](docs-site/zh/guide/getting-started.md) · [安装](docs-site/zh/guide/install.md) · [桌面应用](docs-site/zh/guide/desktop.md) · [无头守护进程](docs-site/zh/guide/daemon.md)

## For developers

**Implementation stack:** Rust workspace (`cargo`); async runtime **Tokio**; Markdown **pulldown-cmark**; i18n **Fluent** (`fluent-bundle`). Runtime is split across crates such as `anycode-core`, `anycode-agent`, `anycode-llm`, `anycode-channel-bridge`, and `anycode-tools` (MCP/LSP).

```bash
cargo fmt
cargo clippy
cargo test --workspace
cargo build --release -p anycode-channel-bridge
cargo build --release -p anycode-desktop
```

Preview the docs site locally:

```bash
cd docs-site && npm install && npm run dev
```
