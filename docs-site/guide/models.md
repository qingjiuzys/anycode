---
title: Models & endpoints
description: LLM providers, config.json fields, z.ai and Anthropic endpoints.
summary: provider, plan, base_url, model, retries, and optional OpenAI feature.
read_when:
  - You configure or switch LLM vendors.
---

# Models & endpoints

Runtime LLM behavior comes from `~/.anycode/config.json` (and per-agent overrides in `routing.agents`). Cargo feature `openai` on `anycode-llm` adds an `OpenAIClient` code path; **OpenAI-compatible gateways** (z.ai, OpenRouter, etc.) still typically use the **Zai** OpenAI-shaped client unless `provider` is exactly `openai` with that feature enabled.

## OpenClaw alignment

- **Provider list**: The CLI and config validator use the static catalog in [`crates/llm/src/provider_catalog.rs`](https://github.com/qingjiuzys/anycode/blob/main/crates/llm/src/provider_catalog.rs) (`PROVIDER_CATALOG`), aligned with the [OpenClaw Provider Directory](https://docs.molt.bot/providers). Canonical upstream ids live in [openclaw/openclaw](https://github.com/openclaw/openclaw).
- **Model refs**: OpenClaw often writes `provider/model` (e.g. `anthropic/claude-opus-4-6`). In anyCode, split that into **`provider`** + **`model`** in `config.json` (same meaning). You may also put a qualified string in **`model`** alone (e.g. `anthropic/claude-3-5-sonnet`); it is validated independently of the global **`provider`**. Resolution helpers live in **`anycode_llm`** (`build_qualified_chat_model_value`, `resolve_chat_model_ref`, mirroring OpenClaw `chat-model-ref.ts`).
- **`anycode status`**: prints **`primary_chat_ref`**, **`model_routes`** aliases, and the resolved **`provider / model`** per **`RuntimeMode`** so you can verify mode routing without starting a session.
- **Naming**: Config `provider` values are **snake_case** (e.g. `cloudflare_ai_gateway`, `vercel_ai_gateway`). OpenClaw **kebab-case** names are accepted and normalized (e.g. `cloudflare-ai-gateway` → `cloudflare_ai_gateway`).
- **Aliases**: Examples: `claude` → `anthropic`, `zai` / `bigmodel` → `z.ai`, `kimi` → `moonshot`, `github-copilot` → `copilot`, `amazon-bedrock` → `bedrock`, `glm` → `z.ai`.
- **AWS Bedrock**: Set `provider` to `amazon_bedrock` (alias `bedrock`), choose a model id for your region, and rely on the AWS credential chain (e.g. `AWS_PROFILE`, instance role). The stack uses **Bedrock Converse** (`Converse` / streaming).
- **GitHub Copilot**: Set `provider` to `github_copilot` (alias `copilot`), pick a Copilot Chat–compatible model id, then run **`anycode model auth copilot`** (device flow) so tokens are stored under `~/.anycode/credentials/`.
- **Placeholders**: Some catalog entries remain OpenClaw parity only (e.g. media-only APIs). Use **`custom`** with your own OpenAI-compatible `base_url` when the catalog entry is not wired in anyCode.

Run **`anycode model`** to pick a provider interactively; the menu follows the same catalog.

## Validation scope

anyCode ships a broad provider catalog, but **maintainer day-to-day validation** focuses on:

| Tier | Providers | What it means |
|------|-----------|---------------|
| **Maintainer-validated** | **z.ai / GLM** (default, e.g. `glm-5`), **DeepSeek** (OpenAI-compatible, including tool-schema normalization) | Primary chat, tool calls, and streaming paths are exercised regularly during development. |
| **Automated in CI** | Mock OpenAI-compatible server only | Agent loop E2E (`cli_e2e_mock_llm`) covers orchestration without calling live vendor APIs. |
| **Catalog-supported** | Anthropic, Bedrock, Copilot, OpenRouter, Ollama, `custom`, and other catalog entries | Configurable via `config.json`; model compatibility varies by endpoint—verify after you add credentials. |

**How to self-test** after configuring a provider:

1. Run **`anycode status`** — check resolved `provider / model` and routes.
2. Open **Workbench → Settings → Model & routing** and use **Test** (`POST /api/settings/models/{id}/test`).
3. Run a short chat: `anycode run --agent general-purpose "Reply with OK only"`.

Default config uses **`provider: z.ai`** and **`model: glm-5`**. DeepSeek has a built-in model catalog (`deepseek-v4-pro`, `deepseek-v4-flash`, legacy `deepseek-chat` / `deepseek-reasoner`) and a quick-auth preset in `anycode setup`.

## DeepSeek (OpenAI-compatible)

Set `provider` to `deepseek` (aliases such as `deep-seek` normalize automatically). Provide `api_key` and a model id from the built-in catalog or your endpoint docs. The stack uses the shared OpenAI Chat Completions client with DeepSeek-specific tool-schema normalization.

Example:

```json
{
  "provider": "deepseek",
  "model": "deepseek-chat",
  "api_key": "YOUR_KEY"
}
```

Use **`anycode model`** or Workbench Settings to pick a preset; then run the self-test steps above.

## `config.json` fields (summary)

- **`provider`**: A known catalog id (see `PROVIDER_CATALOG` above), plus aliases such as `z.ai` / `bigmodel` / `zai`, `anthropic` / `claude`, or kebab-case OpenClaw-style ids.
- **`plan`**: `coding`, `general`, `coding_cn`, or `general_cn` (affects default z.ai base URL when `base_url` is empty).
- **`base_url`**: optional override.
- **`model`**: model id for the active provider.
- **`api_key`**: vendor key.
- **`provider_credentials`**: extra keys for other vendors when routing mixes providers.
- **`session`** (optional): TUI session behavior. **`auto_compact`** (default `true`): before sending your next user message, if the last agent turn reported **input token usage** above a threshold, anyCode runs **automatic compaction** (same pipeline as `/compact`). **`context_window_auto`** (default `true`): derive the context window from **`provider` + `model`** (built-in heuristics in `anycode_llm::resolve_context_window_tokens`, e.g. Claude ≈200k, GLM/z.ai ≈128k, Gemini ≈1M). Set **`context_window_auto`** to `false` and set **`context_window_tokens`** to a fixed size when you want a manual override. Tune **`auto_compact_ratio`** (default `0.88`) or **`auto_compact_min_input_tokens`** (absolute threshold, overrides the ratio). Set **`auto_compact`** to `false` to disable. The runtime flag **`context-compression`** (`**anycode enable context-compression**`) is tracked in **`runtime.features`** (see [Releases & flags](./releases)); threshold behavior remains driven by **`session.auto_compact_*`** fields above.
- **`runtime.max_agent_turns`** / **`runtime.max_tool_calls`** (optional): cap LLM round-trips (default **8**) and cumulative tool executions per task (default **32**). Exceeding tool calls fails the task; exceeding turns ends with a summary. Edit in Workbench **Settings → Agents**, or set env **`ANYCODE_MAX_AGENT_TURNS`** / **`ANYCODE_MAX_TOOL_CALLS`** (env wins over config).

## z.ai (BigModel)

Default endpoints (when `base_url` omitted):

- General: `https://api.z.ai/api/paas/v4/chat/completions`
- Coding plan: `https://api.z.ai/api/coding/paas/v4/chat/completions`

Client uses OpenAI Chat Completions shape: `tools` / `tool_calls` and multi-turn history.

## Anthropic

Set `provider` to `anthropic` (or `claude`), provide `api_key` and a valid `model` id for the Messages API. Optional `base_url` overrides the default endpoint.

## Retries

Retries with backoff on HTTP **429**, **5xx**, and transport errors. **401/403** and other non-retryable codes are not retried.

## OpenAI official API (optional)

With `cargo build -p anycode --features openai`, if global `provider` normalizes to `openai`, the stack may use `OpenAIClient` instead of `ZaiClient` for that profile.

## Dashboard model settings

The Digital Workbench **Settings → Model & routing** section is a **model manager**: configure models once, enable per capability (chat, vision, embedding, STT, TTS, image, video), and switch active models with one click.

API summary:

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/api/settings/models` | Configured model registry (`active` + `items`) |
| `PUT` | `/api/settings/models` | Merge-safe registry update |
| `POST` | `/api/settings/models/{id}/enable` | Set active capability |
| `POST` | `/api/settings/models/{id}/test` | Draft-aware probe |
| `GET` | `/api/settings/model-catalog` | Static + cached provider presets |
| `POST` | `/api/settings/model-catalog/refresh` | Refresh remote catalog cache |
| `GET` | `/api/settings/llm` | Masked flat config + legacy `models.*` |
| `PUT` | `/api/settings/llm` | Patch flat chat, fallback, routing agents |
| `POST` | `/api/settings/llm` | Probe by capability id |

Legacy **`PUT /api/settings/llm`** remains for chat/fallback/routing patches. Prefer **`PUT /api/settings/models`** for the unified registry. **`GET /api/settings/doctor`** includes LLM checks: config file present, `api_key` set, and a warning when `provider` is Google without fallback.

## Unified model registry (`models.active` + `models.items`)

New configs store configured models under **`models.items`** with an **`models.active`** map (capability id → model id). Legacy flat `provider` / `model` and **`models.embedding`**, **`models.speech.*`**, etc. are migrated automatically and kept in sync on save.

Example:

```json
"models": {
  "active": {
    "chat": "zai-glm-5",
    "embedding": "openai-embed-small",
    "stt": "openai-whisper"
  },
  "items": [
    {
      "id": "zai-glm-5",
      "provider": "z.ai",
      "model": "glm-5",
      "capabilities": ["chat"],
      "enabled": true
    }
  ]
}
```

Active **chat** syncs to top-level **`provider`** / **`model`** for CLI compatibility.

## Model failover

Store a secondary chat profile under **`runtime.model_fallback`**:

```json
"runtime": {
  "model_fallback": {
    "provider": "anthropic",
    "model": "claude-sonnet-4-20250514",
    "on": "geo"
  }
}
```

`on` is `geo` (default), `rate_limit`, or `any_error`. The agent runtime switches to the fallback when the trigger matches (see agent failover). Google as the primary provider without fallback is flagged in doctor diagnostics.

## Multimodal `models.*` profiles

Optional top-level **`models`** overrides per capability without changing the main chat `provider` / `model`:

- **`models.active`** — map of capability → configured model id (preferred)
- **`models.items`** — unified registry of configured models
- **`models.embedding`** — memory / RAG embeddings (legacy, synced from registry)
- **`models.speech.stt`** / **`models.speech.tts`** — speech
- **`models.image`** / **`models.video`** — image / video generation

Capabilities: **`chat`**, **`vision`** (multimodal input), **`embedding`**, **`stt`**, **`tts`**, **`image`**, **`video`**.

Each entry uses the same shape as a routing agent profile (`provider`, `model`, `api_key`, `base_url`, …). Video generation may use **`endpoint_overrides.submit`** for custom POST URLs.

## Local presets (vision / embedding / STT / TTS)

The workbench **Settings → Model & routing** panel includes **Local presets**: one-click registry entries for on-device or local-HTTP models. Model weights are **not** bundled in the anycode binary; they download on first use (e.g. FastEmbed → `~/.cache/fastembed`, Whisper/Piper → `~/.anycode/models/`).

| Capability | Built-in (optional feature) | External (zero binary size) |
|------------|---------------------------|-----------------------------|
| **Embedding** | `local_fastembed` + `--features embedding-local` | Ollama `nomic-embed-text` at `http://127.0.0.1:11434/v1` |
| **Vision** | — (uses chat model) | Ollama `llava` with `chat` + `vision` capabilities |
| **STT** | `local_whisper` + `--features stt-local` | `whisper_cpp` HTTP at `http://127.0.0.1:8080/v1` |
| **STT (macOS desktop)** | `apple_speech` via **anyCode.app** (Apple Speech, no model download) | — |
| **TTS** | `local_piper` + `--features tts-local` | `piper` HTTP at `http://127.0.0.1:5000/v1` |

### macOS desktop native STT & OCR

In **anyCode.app** (Tauri shell on macOS), you can enable **Apple Speech (macOS native)** under **Settings → Model & routing → Local presets** instead of whisper.cpp. Voice input in the composer uses the system Speech framework (no ~74MB whisper model). Image attachments show **Extract text**, which runs on-device OCR via Apple Vision (`VNRecognizeTextRequest`).

- Requires **Speech Recognition** and **Microphone** permissions (System Settings → Privacy).
- Browser sessions at `http://127.0.0.1:43180` cannot use `apple_speech`; use whisper or an HTTP STT provider there.
- OCR is desktop-only and does not replace LLM **vision** for image understanding.

Enable all optional local backends:

```bash
cargo build -p anycode --features media-local
```

When you set **embedding** active to `local_fastembed`, anyCode syncs `memory.pipeline.embedding_provider` to `"local"` so memory recall and tools share the same embedding path.

**Vision** does not use a separate runtime route: images are attached to chat messages. Pick a chat model that also has the **`vision`** capability (or enable an Ollama LLaVA preset, which sets both `chat` and `vision` active).

Preset catalog: `GET /api/settings/model-catalog` → `local_presets`.

---

More detail (Chinese, including feature matrix and env vars) lives in [模型与端点（中文）](/zh/guide/models).
