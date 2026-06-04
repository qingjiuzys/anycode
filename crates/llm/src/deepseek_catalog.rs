//! DeepSeek OpenAI-compatible API — aligned with https://api-docs.deepseek.com/zh-cn/

/// Canonical OpenAI-format chat completions URL (official docs).
pub const DEEPSEEK_OPENAI_CHAT_URL: &str = "https://api.deepseek.com/chat/completions";

/// OpenAI-format models list base (GET `/models`).
pub const DEEPSEEK_OPENAI_API_ROOT: &str = "https://api.deepseek.com";

#[derive(Debug, Clone, Copy)]
pub struct DeepSeekModelCatalogEntry {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    /// `current` | `legacy`
    pub tier: &'static str,
}

/// Built-in catalog: official V4 ids + legacy aliases (see API 首次调用 / 模型 & 价格).
pub const DEEPSEEK_MODEL_CATALOG: &[DeepSeekModelCatalogEntry] = &[
    DeepSeekModelCatalogEntry {
        id: "deepseek-v4-pro",
        label: "DeepSeek V4 Pro",
        description: "旗舰 MoE；1M 上下文、Tool Calls、思考模式（thinking）、JSON 输出",
        tier: "current",
    },
    DeepSeekModelCatalogEntry {
        id: "deepseek-v4-flash",
        label: "DeepSeek V4 Flash",
        description: "高速 MoE；1M 上下文、Tool Calls；日常对话默认推荐",
        tier: "current",
    },
    DeepSeekModelCatalogEntry {
        id: "deepseek-chat",
        label: "deepseek-chat（兼容）",
        description: "映射 V4 Flash 非思考模式；2026-07-24 23:59 弃用",
        tier: "legacy",
    },
    DeepSeekModelCatalogEntry {
        id: "deepseek-reasoner",
        label: "deepseek-reasoner（兼容）",
        description: "映射 V4 Flash 思考模式；2026-07-24 23:59 弃用",
        tier: "legacy",
    },
];

pub fn catalog_entry_for_id(id: &str) -> Option<DeepSeekModelCatalogEntry> {
    let t = id.trim();
    DEEPSEEK_MODEL_CATALOG
        .iter()
        .find(|e| e.id.eq_ignore_ascii_case(t))
        .copied()
}

pub fn is_known_deepseek_model_id(id: &str) -> bool {
    catalog_entry_for_id(id).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_deepseek_ids() {
        assert!(is_known_deepseek_model_id("deepseek-v4-pro"));
        assert!(is_known_deepseek_model_id(" DEEPSEEK-REASONER "));
    }
}
