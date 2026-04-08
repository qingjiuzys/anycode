//! Google Gemini（OpenAI 兼容 `generativelanguage.googleapis.com`）常用模型 id，与 OpenClaw / Google AI Studio 命名对齐。

#[derive(Debug, Clone, Copy)]
pub struct GoogleModelCatalogEntry {
    pub id: &'static str,
    pub label: &'static str,
}

/// 用于向导与校验子集；未知 id 仍可通过「自定义」写入 config。
pub const GOOGLE_MODEL_CATALOG: &[GoogleModelCatalogEntry] = &[
    GoogleModelCatalogEntry {
        id: "gemini-2.5-pro",
        label: "Gemini 2.5 Pro",
    },
    GoogleModelCatalogEntry {
        id: "gemini-2.5-flash",
        label: "Gemini 2.5 Flash",
    },
    GoogleModelCatalogEntry {
        id: "gemini-2.0-flash",
        label: "Gemini 2.0 Flash",
    },
    GoogleModelCatalogEntry {
        id: "gemini-2.0-flash-thinking-exp",
        label: "Gemini 2.0 Flash Thinking (exp)",
    },
    GoogleModelCatalogEntry {
        id: "gemini-1.5-pro",
        label: "Gemini 1.5 Pro",
    },
    GoogleModelCatalogEntry {
        id: "gemini-1.5-flash",
        label: "Gemini 1.5 Flash",
    },
];

pub fn is_known_google_model_id(id: &str) -> bool {
    let t = id.trim();
    GOOGLE_MODEL_CATALOG
        .iter()
        .any(|e| e.id.eq_ignore_ascii_case(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_ids_trim_and_case_insensitive() {
        assert!(is_known_google_model_id(" gemini-2.5-pro "));
        assert!(is_known_google_model_id("GEMINI-2.5-FLASH"));
        assert!(!is_known_google_model_id("custom-model"));
    }

    #[test]
    fn catalog_has_expected_entries() {
        assert!(GOOGLE_MODEL_CATALOG
            .iter()
            .any(|e| e.id == "gemini-2.5-pro"));
    }
}
