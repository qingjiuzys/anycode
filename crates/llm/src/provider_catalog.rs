//! OpenClaw 风格的模型/鉴权提供方目录（静态表，Rust 单一事实来源）。
//!
//! **对照基准**：[OpenClaw Provider Directory](https://docs.molt.bot/providers)；canonical `providerId` 以
//! [openclaw/openclaw](https://github.com/openclaw/openclaw) 插件为准。config 中 `provider` 使用 **snake_case**
//! 规范 id；OpenClaw 文档常见的 **kebab-case** 经 [`normalize_provider_id`] 映射到同一 id。
//! 运行时传输见 [`LlmTransport`]。非 LLM（转写/图像/视频等）不列入本表，避免与 Chat 协议混用。

/// 底层协议形态（决定运行时选用的客户端栈）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmTransport {
    /// OpenAI Chat Completions 兼容（Bearer），含 z.ai / OpenRouter / Ollama /v1 等。
    OpenAiChatCompletions,
    /// Anthropic Messages API（`x-api-key`）。
    AnthropicMessages,
    /// Amazon Bedrock Converse / ConverseStream（AWS 凭证链 + region）。
    BedrockConverse,
    /// GitHub Copilot：用 GitHub token 换 Copilot token，再以 Anthropic Messages 兼容路径调用（Claude 系）。
    GithubCopilot,
}

/// 写入 `config.json` 的 `provider` 字段时使用的规范 id（小写、稳定）。
#[derive(Debug, Clone, Copy)]
pub struct ProviderCatalogEntry {
    pub id: &'static str,
    pub label: &'static str,
    pub hint: Option<&'static str>,
    pub transport: LlmTransport,
    /// OpenAI 兼容网关的推荐 base（`.../chat/completions`）；其它传输可为 `None`。
    pub suggested_openai_base: Option<&'static str>,
    /// 若为 true，CLI 仅展示说明，不写入有效凭据。
    pub placeholder_only: bool,
}

/// Z.AI 子菜单（对齐 OpenClaw「Z.AI auth method」文案），映射到 `plan` + 默认端点。
#[derive(Debug, Clone, Copy)]
pub struct ZaiAuthMethodEntry {
    pub label: &'static str,
    pub hint: Option<&'static str>,
    /// `coding` / `general`
    pub plan: &'static str,
}

pub const ZAI_AUTH_METHODS: &[ZaiAuthMethodEntry] = &[
    ZaiAuthMethodEntry {
        label: "CN",
        hint: Some("国内通用端点"),
        plan: "general",
    },
    ZaiAuthMethodEntry {
        label: "Coding-Plan-CN",
        hint: Some("国内编码套餐"),
        plan: "coding",
    },
    ZaiAuthMethodEntry {
        label: "Coding-Plan-Global",
        hint: Some("编码套餐（与 CN 同端点模型，按 z.ai 账号区域）"),
        plan: "coding",
    },
    ZaiAuthMethodEntry {
        label: "Global",
        hint: Some("国际通用端点"),
        plan: "general",
    },
    ZaiAuthMethodEntry {
        label: "Z.AI API key",
        hint: Some("API Key，套餐由下方 plan/端点决定"),
        plan: "coding",
    },
];

/// 按 **label 字典序**（与 OpenClaw Provider Directory 展示顺序一致），便于与上游心智一致。
pub const PROVIDER_CATALOG: &[ProviderCatalogEntry] = &[
    ProviderCatalogEntry {
        id: "alibaba",
        label: "Alibaba Model Studio",
        hint: Some("阿里云百炼等 OpenAI 兼容 URL（与 qwen 并存，按账号选）"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "amazon_bedrock",
        label: "Amazon Bedrock",
        hint: Some("Converse API；模型填 foundation model id；凭证走 AWS 默认链，区域见 AWS_REGION"),
        transport: LlmTransport::BedrockConverse,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "anthropic",
        label: "Anthropic",
        hint: Some("Claude Messages API + API key"),
        transport: LlmTransport::AnthropicMessages,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "byteplus",
        label: "BytePlus",
        hint: Some("OpenAI 兼容 Chat Completions"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "chutes",
        label: "Chutes",
        hint: Some("OpenAI 兼容"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "cloudflare_ai_gateway",
        label: "Cloudflare AI Gateway",
        hint: Some("填入网关 Chat Completions URL"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "custom",
        label: "Custom Provider",
        hint: Some("任意 OpenAI 兼容 endpoint"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "deepseek",
        label: "DeepSeek",
        hint: Some("OpenAI 兼容"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: Some("https://api.deepseek.com/v1/chat/completions"),
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "fireworks",
        label: "Fireworks",
        hint: Some("OpenAI 兼容"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: Some("https://api.fireworks.ai/inference/v1/chat/completions"),
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "github_copilot",
        label: "GitHub Copilot",
        hint: Some("GitHub PAT 或 `anycode model auth copilot`；模型请选含 claude 的 Copilot 模型 id"),
        transport: LlmTransport::GithubCopilot,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "google",
        label: "Google",
        hint: Some("Gemini OpenAI 兼容层或自建代理 URL"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "groq",
        label: "Groq",
        hint: Some("LPU 推理，OpenAI 兼容"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: Some("https://api.groq.com/openai/v1/chat/completions"),
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "huggingface",
        label: "Hugging Face",
        hint: Some("Inference API / Router，OpenAI 兼容 URL"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "kilocode",
        label: "Kilo Gateway",
        hint: Some("OpenAI 兼容"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "kimi_code",
        label: "Kimi Code",
        hint: Some("OpenAI 兼容（Kimi 编程向；与 moonshot 同属 Kimi 生态）"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "litellm",
        label: "LiteLLM",
        hint: Some("代理 base_url，一般为 /v1/chat/completions"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: Some("http://127.0.0.1:4000/v1/chat/completions"),
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "minimax",
        label: "MiniMax",
        hint: Some("OpenAI 兼容"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "mistral",
        label: "Mistral AI",
        hint: Some("OpenAI 兼容"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: Some("https://api.mistral.ai/v1/chat/completions"),
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "moonshot",
        label: "Moonshot AI",
        hint: Some("Kimi，OpenAI 兼容；OpenClaw 中 `kimi` 常指向本家"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: Some("https://api.moonshot.cn/v1/chat/completions"),
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "nvidia",
        label: "NVIDIA",
        hint: Some("NIM / build 等 OpenAI 兼容端点"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: Some("https://integrate.api.nvidia.com/v1/chat/completions"),
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "ollama",
        label: "Ollama",
        hint: Some("本地 /v1/chat/completions"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: Some("http://127.0.0.1:11434/v1/chat/completions"),
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "openai",
        label: "OpenAI",
        hint: Some("官方 Chat Completions"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: Some("https://api.openai.com/v1/chat/completions"),
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "opencode",
        label: "OpenCode",
        hint: Some("OpenAI 兼容"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "opencode_go",
        label: "OpenCode Go",
        hint: Some("OpenAI 兼容（与 opencode 区分；base_url 按部署填写）"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "openrouter",
        label: "OpenRouter",
        hint: Some("聚合网关"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: Some("https://openrouter.ai/api/v1/chat/completions"),
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "perplexity",
        label: "Perplexity",
        hint: Some("OpenAI 兼容 Chat API"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: Some("https://api.perplexity.ai/chat/completions"),
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "qianfan",
        label: "Qianfan",
        hint: Some("百度千帆 OpenAI 兼容 URL"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "qwen",
        label: "Qwen",
        hint: Some("DashScope 等 OpenAI 兼容 endpoint"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "sglang",
        label: "SGLang",
        hint: Some("本地 OpenAI 兼容"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: Some("http://127.0.0.1:30000/v1/chat/completions"),
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "stepfun",
        label: "StepFun",
        hint: Some("阶跃星辰 OpenAI 兼容 URL（按控制台填写 base_url）"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "synthetic",
        label: "Synthetic",
        hint: Some("OpenAI 兼容"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "together",
        label: "Together AI",
        hint: Some("OpenAI 兼容"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: Some("https://api.together.xyz/v1/chat/completions"),
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "venice",
        label: "Venice AI",
        hint: Some("OpenAI 兼容"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "vercel_ai_gateway",
        label: "Vercel AI Gateway",
        hint: Some("填入网关 Chat Completions URL"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "volcengine",
        label: "Volcano Engine",
        hint: Some("火山方舟 OpenAI 兼容 URL"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "z.ai",
        label: "Z.AI",
        hint: Some("智谱 BigModel，OpenAI 兼容 + Coding/General"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: None,
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "vllm",
        label: "vLLM",
        hint: Some("本地 OpenAI 兼容"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: Some("http://127.0.0.1:8000/v1/chat/completions"),
        placeholder_only: false,
    },
    ProviderCatalogEntry {
        id: "xai",
        label: "xAI",
        hint: Some("Grok，OpenAI 兼容"),
        transport: LlmTransport::OpenAiChatCompletions,
        suggested_openai_base: Some("https://api.x.ai/v1/chat/completions"),
        placeholder_only: false,
    },
];

/// 规范化 `provider`：trim、小写、kebab→snake，再应用 OpenClaw / 历史别名。
pub fn normalize_provider_id(raw: &str) -> String {
    let s = raw.trim().to_lowercase().replace('-', "_");
    match s.as_str() {
        "bigmodel" | "zai" => "z.ai".to_string(),
        "claude" => "anthropic".to_string(),
        "kimi" => "moonshot".to_string(),
        "github_copilot" | "githubcopilot" => "github_copilot".to_string(),
        "copilot" => "github_copilot".to_string(),
        "amazon_bedrock" | "bedrock" => "amazon_bedrock".to_string(),
        "opencodego" => "opencode_go".to_string(),
        "glm" => "z.ai".to_string(),
        _ => s,
    }
}

pub fn catalog_lookup(id: &str) -> Option<&'static ProviderCatalogEntry> {
    let n = normalize_provider_id(id);
    PROVIDER_CATALOG.iter().find(|e| e.id == n)
}

pub fn transport_for_provider_id(id: &str) -> LlmTransport {
    catalog_lookup(id)
        .map(|e| e.transport)
        .unwrap_or(LlmTransport::OpenAiChatCompletions)
}

pub fn is_known_provider_id(id: &str) -> bool {
    catalog_lookup(id).is_some()
}

/// 按任务路由常用 agent_type（与 `routing.md`、CLI `--agent` 一致）。
pub const ROUTING_AGENT_PRESETS: &[(&'static str, &'static str)] = &[
    ("general-purpose", "主对话 / 通用任务"),
    ("plan", "规划 / 拆解"),
    ("explore", "探索 / 轻量"),
    ("summary", "总结（子任务）"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_kebab_to_snake_matches_catalog() {
        assert_eq!(normalize_provider_id("cloudflare-ai-gateway"), "cloudflare_ai_gateway");
        assert_eq!(normalize_provider_id("vercel-ai-gateway"), "vercel_ai_gateway");
        assert_eq!(normalize_provider_id("opencode-go"), "opencode_go");
        assert!(catalog_lookup("cloudflare-ai-gateway").is_some());
    }

    #[test]
    fn normalize_openclaw_aliases() {
        assert_eq!(normalize_provider_id("kimi"), "moonshot");
        assert_eq!(normalize_provider_id("amazon-bedrock"), "amazon_bedrock");
        assert_eq!(normalize_provider_id("github-copilot"), "github_copilot");
        assert_eq!(normalize_provider_id("glm"), "z.ai");
        assert!(catalog_lookup("amazon_bedrock").is_some());
        assert!(catalog_lookup("groq").is_some());
        assert!(catalog_lookup("fireworks").is_some());
    }

    #[test]
    fn catalog_sorted_by_label() {
        let labels: Vec<_> = PROVIDER_CATALOG.iter().map(|e| e.label).collect();
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        assert_eq!(labels, sorted, "PROVIDER_CATALOG must stay sorted by label for OpenClaw parity");
    }
}
