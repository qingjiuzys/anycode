use crate::GeneralPurposeAgent;
use anycode_core::prelude::*;

#[test]
fn test_agent_type() {
    let agent_type = AgentType::new("test");
    assert_eq!(agent_type.as_str(), "test");
}

#[tokio::test]
async fn test_general_purpose_agent() {
    let config = ModelConfig {
        provider: LLMProvider::Anthropic,
        model: "claude-3-5-sonnet-20241022".to_string(),
        base_url: None,
        temperature: Some(0.7),
        max_tokens: Some(4096),
        api_key: None,
        ..Default::default()
    };

    let agent = GeneralPurposeAgent::new(config);
    assert_eq!(agent.agent_type().as_str(), "general-purpose");
    assert!(!agent.description().is_empty());
}
