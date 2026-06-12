//! Quick-auth preset metadata shared by CLI setup and Dashboard wizard.

use serde_json::{json, Value};

#[derive(Debug, Clone, Copy)]
pub struct QuickAuthChoice {
    pub id: &'static str,
    pub label: &'static str,
    pub provider: &'static str,
    pub plan: &'static str,
    pub default_model: &'static str,
    pub base_url: &'static str,
    pub key_envs: &'static [&'static str],
}

/// Single source of truth for quick-auth presets (CLI + Dashboard).
pub const QUICK_AUTH_CHOICES: &[QuickAuthChoice] = &[
    QuickAuthChoice {
        id: "zai-coding",
        label: "z.ai Coding Plan — Global (api.z.ai)",
        provider: "z.ai",
        plan: "coding",
        default_model: "glm-5",
        base_url: "https://api.z.ai/api/coding/paas/v4/chat/completions",
        key_envs: &["ZAI_API_KEY"],
    },
    QuickAuthChoice {
        id: "zai-coding-cn",
        label: "z.ai / 智谱 国内编码套餐 (open.bigmodel.cn)",
        provider: "z.ai",
        plan: "coding_cn",
        default_model: "glm-5",
        base_url: "https://open.bigmodel.cn/api/coding/paas/v4/chat/completions",
        key_envs: &["ZAI_API_KEY"],
    },
    QuickAuthChoice {
        id: "zai-general",
        label: "z.ai General — Global (api.z.ai)",
        provider: "z.ai",
        plan: "general",
        default_model: "glm-5",
        base_url: "https://api.z.ai/api/paas/v4/chat/completions",
        key_envs: &["ZAI_API_KEY"],
    },
    QuickAuthChoice {
        id: "deepseek-api-key",
        label: "DeepSeek API Key",
        provider: "deepseek",
        plan: "general",
        default_model: "deepseek-v4-pro",
        base_url: "https://api.deepseek.com/chat/completions",
        key_envs: &["DEEPSEEK_API_KEY"],
    },
    QuickAuthChoice {
        id: "gemini-api-key",
        label: "Google Gemini API Key",
        provider: "google",
        plan: "general",
        default_model: "gemini-2.5-pro",
        base_url: "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
        key_envs: &["GEMINI_API_KEY", "GOOGLE_API_KEY"],
    },
    QuickAuthChoice {
        id: "qwen-api-key",
        label: "Qwen API Key (Global Coding Plan)",
        provider: "qwen",
        plan: "general",
        default_model: "qwen3-coder-plus",
        base_url: "https://coding-intl.dashscope.aliyuncs.com/v1/chat/completions",
        key_envs: &["QWEN_API_KEY", "MODELSTUDIO_API_KEY", "DASHSCOPE_API_KEY"],
    },
    QuickAuthChoice {
        id: "qwen-api-key-cn",
        label: "Qwen API Key (China Coding Plan)",
        provider: "qwen",
        plan: "general",
        default_model: "qwen3-coder-plus",
        base_url: "https://coding.dashscope.aliyuncs.com/v1/chat/completions",
        key_envs: &["QWEN_API_KEY", "MODELSTUDIO_API_KEY", "DASHSCOPE_API_KEY"],
    },
    QuickAuthChoice {
        id: "anthropic-api-key",
        label: "Anthropic API Key",
        provider: "anthropic",
        plan: "general",
        default_model: "claude-sonnet-4-20250514",
        base_url: "https://api.anthropic.com/v1/messages",
        key_envs: &["ANTHROPIC_API_KEY"],
    },
    QuickAuthChoice {
        id: "openai-api-key",
        label: "OpenAI API Key",
        provider: "openai",
        plan: "general",
        default_model: "gpt-4.1",
        base_url: "https://api.openai.com/v1/chat/completions",
        key_envs: &["OPENAI_API_KEY"],
    },
];

pub fn quick_auth_presets() -> Value {
    let presets: Vec<Value> = QUICK_AUTH_CHOICES
        .iter()
        .map(|c| {
            json!({
                "id": c.id,
                "label": c.label,
                "provider": c.provider,
                "plan": c.plan,
                "default_model": c.default_model,
                "base_url": c.base_url,
                "key_envs": c.key_envs,
            })
        })
        .collect();
    Value::Array(presets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_match_choices_count() {
        let value = quick_auth_presets();
        let presets = value.as_array().expect("presets array");
        assert_eq!(presets.len(), QUICK_AUTH_CHOICES.len());
    }
}
