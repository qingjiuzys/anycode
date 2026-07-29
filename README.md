<p align="center">
  <img src="brand/anycode-mark.svg" width="72" alt="anyCode" />
</p>

<h1 align="center">anyCode</h1>

<p align="center">
  <strong>Enterprise-grade agents you control</strong>
</p>

<p align="center">
  Local-first · BYOK · data stays on your machine<br/>
  Run agents on your own hardware — not a black-box cloud gateway.
</p>

<p align="center">
  <a href="README.zh.md">简体中文</a> ·
  <a href="https://anycode.work/docs/">Docs</a> ·
  <a href="https://github.com/qingjiuzys/anycode/releases">Releases</a> ·
  <a href="LICENSE">MIT</a>
</p>

<p align="center">
  <img alt="license" src="https://img.shields.io/badge/license-MIT-blue.svg" />
  <img alt="rust" src="https://img.shields.io/badge/rust-edition%202021-orange.svg" />
  <img alt="platform" src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg" />
  <img alt="status" src="https://img.shields.io/badge/status-invite--only%20beta-yellow.svg" />
</p>

---

## Why anyCode

Most AI coding assistants push reasoning and context into a vendor cloud. anyCode flips that:

| | Hosted agents | **anyCode** |
|---|---|---|
| Where it runs | Vendor servers | **Your machine / private network** |
| Models | Tied to a plan | **BYOK** — any provider or private endpoint |
| Code & files | Uploaded to the cloud | **Stay local by default** |
| Team work | Shared cloud workspace | **Explicit handoff** after peer approval |
| Control | Opaque policy | **Approvals, policies, Skills, REST API** |

Built for teams that need **sovereignty, compliance, and extensibility**.

---

## What's new in v0.3.0

This release focuses on **alignment before action** and **team collaboration**:

### Grill Me（拷问）

Turn on Grill mode (`/grill-me` or `/拷问`). The agent **aligns first, implements later**:

- One question at a time via `AskUserQuestion`
- Every question includes a recommended option
- Repo-answerable questions are answered from the codebase, not the user
- Implementation starts only when you say so (e.g. “go ahead” / 「可以动手了」)

Stops the “rewrite the repo before we agree on the goal” failure mode.

### Team Handoff（多人协作交接）

Hand a project or session to a colleague — not a zip dump or screenshot:

- **LAN**: mDNS discovery + peer-to-peer transfer ([ADR 015](docs/adr/015-lan-colleague-handoff.md))
- **Cloud team**: same-org devices via A2A streaming relay ([ADR 016](docs/adr/016-cloud-a2a-team-handoff.md)) — **no OSS**; bundle bytes stay in an in-memory pipe
- **Explicit consent**: recipient approves in Desktop before a stream token is issued
- Portal **Team** page lists org members and online Desktop instances

---

## Features

### Local Digital Workbench

Projects, sessions, assets, automations, and security approvals — all on a loopback Workbench.  
Bundled in **anyCode.app** (macOS), or at `http://127.0.0.1:43180` in dev builds.

### Single Rust Agent Runtime

One `AgentRuntime` owns the multi-turn LLM + tool loop — Bash, Edit, Grep, MCP, LSP, Skills, Cron, Knowledge, and more. Not a thin shell over a remote gateway.

### BYOK model catalog

30+ providers (z.ai/GLM, DeepSeek, Anthropic, Bedrock, Copilot, OpenRouter, Ollama, custom OpenAI-compatible endpoints…). Keys stay in `~/.anycode/config.json`.

> Maintainer day-to-day validation: **z.ai / GLM** and **DeepSeek**. Other catalog entries are configuration-supported; CI uses a local mock, not live vendor APIs.

### Grill Me & Goal mode

- **Grill Me**: Socratic plan alignment before any edits  
- **Goal mode**: iterate autonomously until gates / goals are met  

### Team handoff (LAN + Cloud A2A)

Discover peers → request handoff → peer approves → stream a `handoff_v1` bundle. Semantics align with [Google A2A](https://google.github.io/A2A/) concepts while keeping the “data stays local” posture.

### Built-in scheduler

Natural-language cron via `anycode-daemon scheduler`, with run history, guardrails, and Workbench session output.

### Skills & office deliverables

Installable Skills (including doc / spreadsheet / deck / PDF pipelines) — agents that ship artifacts, not only code.

### macOS-native extras

**Apple Speech** (no Whisper download) and **Apple Vision OCR** in the Desktop shell. Browser-only Workbench on loopback does not include these.

### Enterprise-friendly extension surface

Local REST API, API tokens, project policies, documented permission modes, eval/gate harness. SSO/RBAC is on the roadmap.

---

## Quick start

1. Install **anyCode.app** (macOS) or **`anycode-daemon`** (Linux / Windows headless)
2. Open Workbench **`/setup`** and configure a model
3. Send a test message — try `/grill-me` or hand off to a colleague

### macOS (recommended)

Download `anyCode_<version>_aarch64.dmg` from [Releases](https://github.com/qingjiuzys/anycode/releases) and drag **anyCode** into Applications.

### Linux / Windows (headless)

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

```powershell
irm https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.ps1 | iex
```

Then open `http://127.0.0.1:43180/setup`.

**Verify:** send “Reply with OK only”.

---

## Architecture at a glance

```text
┌─────────────────────────────────────────────────────────┐
│  anyCode.app / Browser                                  │
│  Digital Workbench  ·  Grill / Goal  ·  Colleague UI    │
└───────────────────────────┬─────────────────────────────┘
                            │ loopback :43180
┌───────────────────────────▼─────────────────────────────┐
│  AgentRuntime (Rust / Tokio)                            │
│  LLM providers · Tools · Skills · Approvals · Memory    │
└───────────────┬─────────────────────────┬───────────────┘
                │                         │
        ~/.anycode/                 anycode-daemon
        config · sessions           scheduler (cron)
                │
                ▼ optional cloud (account / A2A signaling)
        anycode.work — device link · org peers · streaming handoff
                       (bundle never lands in OSS / DB)
```

---

## Documentation

| | |
|---|---|
| Getting started | [guide](https://anycode.work/docs/guide/getting-started) |
| Install | [install](https://anycode.work/docs/guide/install) |
| Desktop (macOS) | [desktop](https://anycode.work/docs/guide/desktop) |
| Models & endpoints | [models](https://anycode.work/docs/guide/models) |
| Workbench | [workbench](https://anycode.work/docs/guide/workbench) |
| Scheduled jobs | [scheduler](https://anycode.work/docs/guide/cli-scheduler) |
| Troubleshooting | [troubleshooting](https://anycode.work/docs/guide/troubleshooting) |

Maintainer docs: [`docs/`](docs/) · ADRs · [`docs/roadmap.md`](docs/roadmap.md)

---

## Develop

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace

# Desktop iteration
./scripts/sync-desktop-dev.sh          # UI only
./scripts/sync-desktop-dev.sh --rust   # UI + Rust (release-local)
```

Preview user docs locally:

```bash
cd crates/account-portal && npm install && npm run dev
# → http://127.0.0.1:43201/docs
```

Stack: Rust workspace · Tokio · React (Workbench) · Tauri (Desktop) · Fluent i18n.

---

## Status & license

Invite-only beta while algorithm-filing review is in progress. Do not treat this as general availability or regulatory approval.

License: [MIT](LICENSE)

---

<p align="center">
  <sub>Enterprise agents you control — local execution, explicit handoff, grill before you build.</sub>
</p>
