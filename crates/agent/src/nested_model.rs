//! Claude Code `model: sonnet | opus | haiku` → concrete `ModelConfig` for nested runs.

use anycode_core::{LLMProvider, ModelConfig};

/// Apply a nested-agent model hint on top of the parent session config.
///
/// Recognizes **`sonnet`**, **`opus`**, **`haiku`** (case-insensitive). For **`Anthropic`**
/// provider, uses Messages API model ids; for OpenAI-compatible gateways, uses
/// **`anthropic/<id>`** qualified refs. Any other non-empty string sets **`model`** verbatim.
pub fn resolve_nested_model_hint(base: &ModelConfig, hint: &str) -> ModelConfig {
    let mut out = base.clone();
    let h = hint.trim();
    if h.is_empty() {
        return out;
    }
    let lower = h.to_ascii_lowercase();
    let family = match lower.as_str() {
        "sonnet" => Some("claude-sonnet-4-5-20250929"),
        "opus" => Some("claude-opus-4-5-20250929"),
        "haiku" => Some("claude-haiku-4-5-20251001"),
        _ => None,
    };
    if let Some(mid) = family {
        match &base.provider {
            LLMProvider::Anthropic => {
                out.model = mid.to_string();
            }
            LLMProvider::OpenAI | LLMProvider::Local | LLMProvider::Custom(_) => {
                out.model = format!("anthropic/{mid}");
            }
        }
        return out;
    }
    out.model = h.to_string();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_core::LLMProvider;

    #[test]
    fn anthropic_sonnet_hint() {
        let base = ModelConfig {
            provider: LLMProvider::Anthropic,
            model: "claude-opus-4-5-20250929".into(),
            base_url: None,
            temperature: None,
            max_tokens: None,
            api_key: None,
        };
        let o = resolve_nested_model_hint(&base, "sonnet");
        assert_eq!(o.model, "claude-sonnet-4-5-20250929");
    }

    #[test]
    fn openai_compat_qualified() {
        let base = ModelConfig {
            provider: LLMProvider::OpenAI,
            model: "gpt-4o".into(),
            base_url: Some("https://example/v1".into()),
            temperature: None,
            max_tokens: None,
            api_key: None,
        };
        let o = resolve_nested_model_hint(&base, "haiku");
        assert_eq!(o.model, "anthropic/claude-haiku-4-5-20251001");
    }
}
