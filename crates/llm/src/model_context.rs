//! 模型上下文窗口（token）的**启发式**解析，用于 TUI 自动压缩阈值等。
//!
//! 上游 API 通常不在静态配置里暴露「窗口上限」，因此这里用 **provider + model id 子串**
//! 做保守估计；未知组合回退到 [`DEFAULT_CONTEXT_WINDOW_TOKENS`]。
//! 用户可在 `config.json` 的 `session.context_window_auto: false` 时改用手动值覆盖。

/// 无法识别模型时的回退窗口（与常见 128k 级模型对齐）。
pub const DEFAULT_CONTEXT_WINDOW_TOKENS: u32 = 128_000;

pub(crate) fn resolve_known_context_window_tokens(
    normalized_provider_id: &str,
    model_id: &str,
) -> u32 {
    let m = model_id.to_ascii_lowercase();
    let p = normalized_provider_id.to_ascii_lowercase();

    if m.contains("gemini") {
        return 1_000_000;
    }
    if m.contains("claude") || m.contains("opus") || m.contains("sonnet") || m.contains("haiku") {
        return 200_000;
    }
    if m.contains("gpt-4")
        || m.contains("gpt-5")
        || m.contains("gpt-4o")
        || m.contains("o1")
        || m.contains("o3")
        || m.contains("o4")
    {
        return 128_000;
    }
    if m.contains("gpt-3.5") {
        return 16_384;
    }
    if m.contains("glm") || m.contains("qwen") || m.contains("deepseek") {
        return 128_000;
    }
    if m.contains("llama") || m.contains("mistral") || m.contains("mixtral") {
        return 128_000;
    }

    match p.as_str() {
        "anthropic" | "claude" => 200_000,
        "amazon_bedrock" | "bedrock" => 200_000,
        "github_copilot" | "copilot" => 200_000,
        "openai" => 128_000,
        "google" => 1_000_000,
        "z.ai" | "zai" | "bigmodel" => 128_000,
        _ => DEFAULT_CONTEXT_WINDOW_TOKENS,
    }
}

/// 根据规范化后的 `provider` id（见 [`crate::normalize_provider_id`]）与 `model` 字符串推断上下文窗口上限（tokens）。
///
/// 匹配顺序：模型名子串（更具体）→ 厂商默认 → 全局默认。
pub fn resolve_context_window_tokens(normalized_provider_id: &str, model_id: &str) -> u32 {
    crate::resolve_runtime_model_capabilities(normalized_provider_id, model_id, None).context_tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_model_uses_200k() {
        assert_eq!(
            resolve_context_window_tokens("anthropic", "claude-sonnet-4-5-20250929"),
            200_000
        );
    }

    #[test]
    fn glm_uses_128k() {
        assert_eq!(resolve_context_window_tokens("z.ai", "glm-5"), 128_000);
    }

    #[test]
    fn gemini_name_hits_1m() {
        assert_eq!(
            resolve_context_window_tokens("openrouter", "google/gemini-2.0-flash"),
            1_000_000
        );
    }

    #[test]
    fn unknown_model_falls_back_to_provider_or_default() {
        assert_eq!(
            resolve_context_window_tokens("anthropic", "custom-model"),
            200_000
        );
        assert_eq!(resolve_context_window_tokens("acme_unknown", "x"), 128_000);
    }

    #[test]
    fn minicpm_and_ollama_do_not_fall_back_to_128k() {
        assert_eq!(
            resolve_context_window_tokens("openai", "managed-minicpm5-1b"),
            4_096
        );
        assert_eq!(
            resolve_context_window_tokens("ollama", "minicpm5-1b-e2e"),
            32_768
        );
        assert_eq!(
            resolve_context_window_tokens("ollama", "custom-local"),
            4_096
        );
    }
}
