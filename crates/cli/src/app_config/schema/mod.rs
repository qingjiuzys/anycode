//! Config schema/defaults/helpers extracted from `app_config.rs` to keep orchestration logic smaller.

mod types;
pub(crate) use types::*;

mod validation;
pub(crate) use validation::*;

use anycode_llm::ZAI_MODEL_CATALOG;

pub(crate) fn is_known_zai_model(model: &str) -> bool {
    let m = model.trim();
    if m.is_empty() {
        return false;
    }
    ZAI_MODEL_CATALOG.iter().any(|e| e.api_name == m)
}

pub(crate) fn is_zai_family_provider(p: &str) -> bool {
    matches!(p.trim(), "z.ai" | "zai" | "bigmodel")
}

pub(crate) fn is_anthropic_family_provider(p: &str) -> bool {
    matches!(p.trim(), "anthropic" | "claude")
}
