---
title: 模型与端点
description: z.ai、Anthropic、config.json 与可选 openai feature。
summary: 运行时配置与默认端点、重试与 OpenAI 官方路径说明。
read_when:
  - 配置或切换 LLM 提供商。
---

# 模型与端点

## 运行时配置 vs 编译期 `openai` feature

- `**anycode model` 向导与 `config.json**`：决定运行时用哪家 API、密钥与 `base_url`。凡走 **OpenAI Chat Completions 兼容** 传输时，默认使用 `**ZaiClient`**（`build_zai_openai_stack_client` / `MultiProviderLlmClient`）。例外：编译启用 `**openai`** feature 且全局 `provider` 为规范 id `**openai**` 时，该分支改用 `**OpenAIClient**`（见下文）。
- **Cargo feature `openai`**：仅额外编译 `[OpenAIClient](../../../crates/llm/src/providers/openai.rs)`，供需要独立实现或链接可选代码路径时使用；与向导是否填写 OpenAI 官方 URL **无必然绑定**。默认 `anycode` 二进制未对该 feature 启用 `anycode-llm` 的 `openai`（见下文「主 CLI」）。

## 与 OpenClaw 对齐

- **厂商目录**：与 [OpenClaw Provider Directory](https://docs.molt.bot/providers) 对齐的静态表在源码 [`crates/llm/src/provider_catalog.rs`](../../../crates/llm/src/provider_catalog.rs) 的 **`PROVIDER_CATALOG`**；上游 canonical 以 [openclaw/openclaw](https://github.com/openclaw/openclaw) 为准。
- **模型引用**：OpenClaw 常用 `provider/model`（如 `anthropic/claude-opus-4-6`）；anyCode 在 `config.json` 里拆成 **`provider`** + **`model`** 两字段，语义相同。
- **命名**：配置文件里 `provider` 为 **snake_case**（如 `cloudflare_ai_gateway`）。OpenClaw 文档里的 **kebab-case** 会自动规范化（如 `cloudflare-ai-gateway` → `cloudflare_ai_gateway`）。
- **别名示例**：`claude` → `anthropic`；`zai` / `bigmodel` → `z.ai`；`kimi` → `moonshot`；`github-copilot` → `copilot`；`amazon-bedrock` → `bedrock`；`glm` → `z.ai`。
- **AWS Bedrock**：`provider` 设为 `amazon_bedrock`（别名 `bedrock`），填写区域下可用模型 id，凭证走 AWS 链（如 `AWS_PROFILE`、实例角色）。运行时使用 **Bedrock Converse**（含流式）。
- **GitHub Copilot**：`provider` 设为 `github_copilot`（别名 `copilot`），选择兼容 Copilot Chat 的模型 id，并执行 **`anycode model auth copilot`**（设备码登录），令牌写入 `~/.anycode/credentials/`。
- **占位项**：部分目录项仅为与 OpenClaw 一致（例如部分媒体类 API），若 anyCode 未接线，可改用 **`custom`** + 自建 OpenAI 兼容 `base_url`。

完整厂商列表请用 **`anycode model`** 交互菜单，或直接查阅源码中的 `PROVIDER_CATALOG`。

## 配置字段（`~/.anycode/config.json`）

- `provider`：须为目录中的规范 id（及别名）。除 z.ai / Anthropic 外，另有 OpenRouter、Groq、DeepSeek、Bedrock、GitHub Copilot 等，见上节。
- `plan`：`coding` 或 `general`（主要影响 z.ai 默认 `base_url` 选择）
- `base_url`：可选。为空时 z.ai 会按 `plan` 使用默认端点；Anthropic 默认为官方 Messages API，也可覆盖
- `model`：例如 z.ai 的 `glm-5`；其它厂商填对应 API 的模型 id（见厂商文档）
- `api_key`：对应厂商的密钥
- `session`（可选）：TUI 会话。`auto_compact`（默认 `true`）：在发送下一条用户消息前，若上一轮 agent turn 上报的 **input tokens** 超过阈值，则先自动执行与 `/compact` 相同的压缩。`context_window_auto`（默认 `true`）：根据 **`provider` + `model`** 自动推断上下文窗口（见 `anycode_llm::resolve_context_window_tokens`，如 Claude 约 200k、GLM/z.ai 约 128k、Gemini 约 1M）。若需固定窗口，设 `context_window_auto: false` 并填写 `context_window_tokens`。另可用 `auto_compact_ratio`（默认 `0.88`）或 `auto_compact_min_input_tokens`（绝对阈值，优先于比例）。`auto_compact: false` 可关闭自动压缩。

详见 README 中的路由（`routing.agents`）与安全（`security`）字段说明。

## z.ai（BigModel）

### 端点

- **通用**：`https://api.z.ai/api/paas/v4/chat/completions`
- **编码**：`https://api.z.ai/api/coding/paas/v4/chat/completions`

### 工具调用（OpenAI 兼容）

客户端按 **OpenAI Chat Completions** 形态下发 `tools` / `tool_choice`，并解析响应中的 `tool_calls`。具体字段以线上 API 为准；若与标准形态不一致，需在 `crates/llm/src/providers/zai.rs` 中调整解析。

## Anthropic

- 使用 `provider`: `anthropic`（或兼容别名 `claude`）
- `api_key`：Anthropic API Key
- `model`：Anthropic Messages API 支持的模型 id
- `base_url`：可选，覆盖默认 Messages API 地址

## OpenAI 官方 API（可选 crate feature）

- **库**：`anycode-llm` 在启用 Cargo feature `**openai`** 时编译 `OpenAIClient`（实现见 `[crates/llm/src/providers/openai.rs](../../../crates/llm/src/providers/openai.rs)`），默认 URL 为 `https://api.openai.com/v1/chat/completions`；请求/响应与 z.ai 所用 OpenAI Chat Completions 形态一致；`ModelConfig.model` 为空时默认 `gpt-4o-mini`。
- **与 Zai 栈的区别**：配置里 OpenAI **兼容** 网关（z.ai、OpenRouter 等）仍走现有 `ZaiClient` + `build_zai_openai_stack_client`；`OpenAIClient` 面向官方端点或需独立 HTTP 语义时的集成。
- **环境变量**：`ANYCODE_OPENAI_TOOL_CHOICE` 可为 `auto` / `required` / `none`（有工具时），与 z.ai 的 `ANYCODE_ZAI_TOOL_CHOICE` 分离。
- **主 CLI**：`cargo build -p anycode --features openai` 启用；`build_multi_llm_stack` 在全局 `provider` 规范 id 为 `**openai`** 时用 `OpenAIClient`，其它 OpenAI **兼容** 厂商仍用 `ZaiClient`（与上表一致）。

## 重试策略

anyCode 对以下情况做指数退避重试：

- `HTTP 429`（Rate limit）
- `HTTP 5xx`（服务端错误）
- 网络请求错误（发送失败）

不重试：鉴权类错误（例如 401/403）以及其他非 retryable 状态码。