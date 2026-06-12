use anycode_llm::{is_known_provider_id, normalize_provider_id, string_field};
use serde_json::Value;

fn has_non_empty_secret(v: &str) -> bool {
    !v.trim().is_empty()
}

/// Whether `config.json` has enough LLM fields to run a chat (matches CLI `anycode setup` skip logic).
pub fn has_usable_model_config(cfg: &Value) -> bool {
    let provider = string_field(cfg, "provider", "provider").unwrap_or_default();
    let model = string_field(cfg, "model", "model").unwrap_or_default();
    if provider.trim().is_empty() || model.trim().is_empty() {
        return false;
    }
    let norm = normalize_provider_id(&provider);
    if !is_known_provider_id(&norm) {
        return false;
    }
    if string_field(cfg, "api_key", "api_key").is_some_and(|k| has_non_empty_secret(&k)) {
        return true;
    }
    cfg.get("provider_credentials")
        .and_then(|v| v.as_object())
        .is_some_and(|m| {
            m.values()
                .filter_map(|v| v.as_str())
                .any(has_non_empty_secret)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_empty_key() {
        let cfg = json!({
            "provider": "z.ai",
            "model": "glm-5",
            "api_key": ""
        });
        assert!(!has_usable_model_config(&cfg));
    }

    #[test]
    fn accepts_provider_credentials() {
        let cfg = json!({
            "provider": "openai",
            "model": "gpt-4o",
            "api_key": "",
            "provider_credentials": { "openai": "sk-test" }
        });
        assert!(has_usable_model_config(&cfg));
    }

    #[test]
    fn accepts_primary_api_key() {
        let cfg = json!({
            "provider": "z.ai",
            "model": "glm-5",
            "api_key": "secret"
        });
        assert!(has_usable_model_config(&cfg));
    }
}
