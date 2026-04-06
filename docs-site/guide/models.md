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
- **Model refs**: OpenClaw often writes `provider/model` (e.g. `anthropic/claude-opus-4-6`). In anyCode, split that into **`provider`** + **`model`** in `config.json` (same meaning).
- **Naming**: Config `provider` values are **snake_case** (e.g. `cloudflare_ai_gateway`, `vercel_ai_gateway`). OpenClaw **kebab-case** names are accepted and normalized (e.g. `cloudflare-ai-gateway` → `cloudflare_ai_gateway`).
- **Aliases**: Examples: `claude` → `anthropic`, `zai` / `bigmodel` → `z.ai`, `kimi` → `moonshot`, `github-copilot` → `copilot`, `amazon-bedrock` → `bedrock`, `glm` → `z.ai`.
- **AWS Bedrock**: Set `provider` to `amazon_bedrock` (alias `bedrock`), choose a model id for your region, and rely on the AWS credential chain (e.g. `AWS_PROFILE`, instance role). The stack uses **Bedrock Converse** (`Converse` / streaming).
- **GitHub Copilot**: Set `provider` to `github_copilot` (alias `copilot`), pick a Copilot Chat–compatible model id, then run **`anycode model auth copilot`** (device flow) so tokens are stored under `~/.anycode/credentials/`.
- **Placeholders**: Some catalog entries remain OpenClaw parity only (e.g. media-only APIs). Use **`custom`** with your own OpenAI-compatible `base_url` when the catalog entry is not wired in anyCode.

Run **`anycode model`** to pick a provider interactively; the menu follows the same catalog.

## `config.json` fields (summary)

- **`provider`**: A known catalog id (see `PROVIDER_CATALOG` above), plus aliases such as `z.ai` / `bigmodel` / `zai`, `anthropic` / `claude`, or kebab-case OpenClaw-style ids.
- **`plan`**: `coding` or `general` (affects default z.ai base URL when `base_url` is empty).
- **`base_url`**: optional override.
- **`model`**: model id for the active provider.
- **`api_key`**: vendor key.
- **`provider_credentials`**: extra keys for other vendors when routing mixes providers.
- **`session`** (optional): TUI session behavior. **`auto_compact`** (default `true`): before sending your next user message, if the last agent turn reported **input token usage** above a threshold, anyCode runs **automatic compaction** (same pipeline as `/compact`). **`context_window_auto`** (default `true`): derive the context window from **`provider` + `model`** (built-in heuristics in `anycode_llm::resolve_context_window_tokens`, e.g. Claude ≈200k, GLM/z.ai ≈128k, Gemini ≈1M). Set **`context_window_auto`** to `false` and set **`context_window_tokens`** to a fixed size when you want a manual override. Tune **`auto_compact_ratio`** (default `0.88`) or **`auto_compact_min_input_tokens`** (absolute threshold, overrides the ratio). Set **`auto_compact`** to `false` to disable.

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

---

More detail (Chinese, including feature matrix and env vars) lives in [模型与端点（中文）](/zh/guide/models).
