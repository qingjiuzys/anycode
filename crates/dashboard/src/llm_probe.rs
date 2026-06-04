//! LLM connectivity probes for dashboard settings (via [`ResolvedModelRegistry`]).

use anycode_core::{CoreError, LLMProvider, Message, MessageContent, MessageRole, ModelConfig};
use anycode_llm::{
    build_llm_client,
    capability_catalog::ModelCapability,
    media::{EmbeddingClient, ImageGenClient, MediaClientRegistry, TtsClient, VideoGenClient},
    ProviderConfig, ResolvedModelRegistry,
};
use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

/// Routes capability probes through a resolved model registry.
#[derive(Debug, Clone)]
pub struct LlmProbeService {
    registry: ResolvedModelRegistry,
}

impl LlmProbeService {
    pub fn from_config(cfg: &Value) -> Self {
        Self {
            registry: ResolvedModelRegistry::from_config(cfg),
        }
    }

    pub fn from_registry(registry: ResolvedModelRegistry) -> Self {
        Self { registry }
    }

    pub async fn probe(&self, capability: ModelCapability) -> Result<String, String> {
        probe_registry(&self.registry, capability).await
    }
}

async fn probe_registry(
    registry: &ResolvedModelRegistry,
    capability: ModelCapability,
) -> Result<String, String> {
    match capability {
        ModelCapability::Chat | ModelCapability::Vision => probe_chat(registry).await,
        ModelCapability::Embedding => probe_embedding(registry).await,
        ModelCapability::Stt => probe_stt(registry).await,
        ModelCapability::Tts => probe_tts(registry).await,
        ModelCapability::ImageGen => probe_image(registry).await,
        ModelCapability::VideoGen => probe_video(registry).await,
        ModelCapability::Rerank => Err("rerank probe not implemented".into()),
    }
}

fn chat_provider_config(registry: &ResolvedModelRegistry) -> Result<ProviderConfig, String> {
    let cap = ModelCapability::Chat;
    let item = registry
        .active_item(cap)
        .ok_or_else(|| "chat model not configured".to_string())?;
    let provider = registry.resolve_provider(item);
    let model = registry.resolve_model(item);
    let api_key = registry
        .resolve_api_key(item)
        .ok_or_else(|| "api_key not configured".to_string())?;
    Ok(ProviderConfig {
        provider,
        api_key,
        base_url: registry.resolve_base_url(item),
        model,
        temperature: item.temperature.or(Some(0.0)),
        max_tokens: item.max_tokens.or(Some(16)),
        zai_tool_choice_first_turn: false,
    })
}

async fn probe_chat(registry: &ResolvedModelRegistry) -> Result<String, String> {
    let pc = chat_provider_config(registry)?;
    let client = build_llm_client(&pc)
        .await
        .map_err(|e: CoreError| e.to_string())?;
    let resp = client
        .chat(
            vec![Message {
                id: Uuid::new_v4(),
                role: MessageRole::User,
                content: MessageContent::Text("ping".into()),
                timestamp: Utc::now(),
                metadata: Default::default(),
            }],
            vec![],
            &ModelConfig {
                provider: LLMProvider::Custom(pc.provider.clone()),
                model: pc.model.clone(),
                base_url: pc.base_url.clone(),
                temperature: pc.temperature,
                max_tokens: pc.max_tokens,
                api_key: Some(pc.api_key),
            },
        )
        .await
        .map_err(|e: CoreError| e.to_string())?;
    let preview = match &resp.message.content {
        MessageContent::Text(t) => t.chars().take(80).collect::<String>(),
        _ => "(non-text)".into(),
    };
    Ok(format!("chat ok: {}", preview))
}

async fn probe_embedding(registry: &ResolvedModelRegistry) -> Result<String, String> {
    let reg = MediaClientRegistry::from_registry(registry);
    let prof = reg
        .embedding
        .as_ref()
        .ok_or_else(|| "models.embedding not configured".to_string())?;
    let client = EmbeddingClient::new(prof.profile.clone());
    let dim = client
        .embed("hello")
        .await
        .map_err(|e: CoreError| e.to_string())?
        .len();
    Ok(format!("embedding ok: dim={dim}"))
}

async fn probe_stt(registry: &ResolvedModelRegistry) -> Result<String, String> {
    let reg = MediaClientRegistry::from_registry(registry);
    let prof = reg
        .stt
        .as_ref()
        .ok_or_else(|| "models.speech.stt not configured".to_string())?;
    if prof.profile.base_url.is_none() && prof.profile.provider != "openai" {
        return Err("STT requires base_url or openai provider".into());
    }
    Ok(format!(
        "stt configured: provider={} model={}",
        prof.profile.provider, prof.profile.model
    ))
}

async fn probe_tts(registry: &ResolvedModelRegistry) -> Result<String, String> {
    let reg = MediaClientRegistry::from_registry(registry);
    let prof = reg
        .tts
        .as_ref()
        .ok_or_else(|| "models.speech.tts not configured".to_string())?;
    let client = TtsClient::new(prof.profile.clone());
    let out = client
        .synthesize("ok")
        .await
        .map_err(|e: CoreError| e.to_string())?;
    Ok(format!("tts ok: {} bytes", out.audio_bytes.len()))
}

async fn probe_image(registry: &ResolvedModelRegistry) -> Result<String, String> {
    let reg = MediaClientRegistry::from_registry(registry);
    let prof = reg
        .image
        .as_ref()
        .ok_or_else(|| "models.image not configured".to_string())?;
    if prof.profile.base_url.is_none() && prof.profile.provider != "openai" {
        return Ok(format!(
            "image configured (dry): provider={} model={}",
            prof.profile.provider, prof.profile.model
        ));
    }
    let client = ImageGenClient::new(prof.profile.clone());
    let out = client
        .generate("a small red dot on white")
        .await
        .map_err(|e: CoreError| e.to_string())?;
    Ok(format!(
        "image ok: url={} b64={}",
        out.url.is_some(),
        out.b64_json.is_some()
    ))
}

async fn probe_video(registry: &ResolvedModelRegistry) -> Result<String, String> {
    let reg = MediaClientRegistry::from_registry(registry);
    let prof = reg
        .video
        .as_ref()
        .ok_or_else(|| "models.video not configured".to_string())?;
    if prof.profile.base_url.is_none() {
        return Ok(format!(
            "video configured (needs base_url to probe): model={}",
            prof.profile.model
        ));
    }
    let client = VideoGenClient::new(prof.profile.clone());
    let out = client
        .generate("test clip")
        .await
        .map_err(|e: CoreError| e.to_string())?;
    Ok(format!(
        "video ok: url={} job={:?}",
        out.url.is_some(),
        out.job_id
    ))
}
