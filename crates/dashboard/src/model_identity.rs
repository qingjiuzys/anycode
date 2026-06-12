//! Helpers for identifying eval/test LLM fixtures that should not appear in user-facing stats.

/// True when `model` (or `provider/model`) refers to a local mock LLM used in eval/e2e tests.
#[must_use]
pub fn is_mock_llm_model(model: &str) -> bool {
    let m = model.trim();
    if m.is_empty() {
        return false;
    }
    let lower = m.to_ascii_lowercase();
    lower == "mock" || lower.starts_with("mock/")
}

/// True when the provider id is the dedicated mock/test provider.
#[must_use]
pub fn is_mock_llm_provider(provider: &str) -> bool {
    provider.trim().eq_ignore_ascii_case("mock")
}

/// True when either provider or model identifies a mock/test LLM profile.
#[must_use]
pub fn is_mock_llm_profile(provider: &str, model: &str) -> bool {
    is_mock_llm_provider(provider) || is_mock_llm_model(model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_model_ids() {
        assert!(is_mock_llm_model("mock"));
        assert!(is_mock_llm_model("mock/model"));
        assert!(is_mock_llm_model(" MOCK/model "));
        assert!(is_mock_llm_model("mock/fixture"));
        assert!(!is_mock_llm_model("glm-5"));
        assert!(!is_mock_llm_model(""));
    }

    #[test]
    fn mock_provider_ids() {
        assert!(is_mock_llm_provider("mock"));
        assert!(is_mock_llm_provider("Mock"));
        assert!(!is_mock_llm_provider("openrouter"));
    }
}
