//! Local / on-device media model presets (vision, embedding, STT, TTS).
//!
//! Models are **not** bundled in the anycode binary; first use may download ONNX / voice
//! assets to `~/.anycode/models/` or provider-specific caches.

use crate::capability_catalog::ModelCapability;
use crate::config_models::ConfiguredModelFile;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// How the preset is satisfied at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalMode {
    /// Inference inside anycode (optional compile-time features).
    Builtin,
    /// Connect to a local HTTP service (whisper.cpp server, Piper, Ollama, …).
    External,
    /// macOS platform APIs (Speech / Vision) via desktop shell only.
    PlatformNative,
}

/// A one-click local media preset for the model registry.
#[derive(Debug, Clone, Copy)]
pub struct LocalMediaPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub capabilities: &'static [ModelCapability],
    pub mode: LocalMode,
    pub provider: &'static str,
    pub model: &'static str,
    pub base_url: Option<&'static str>,
    pub api_key: Option<&'static str>,
    pub voice: Option<&'static str>,
    pub docs_url: Option<&'static str>,
    pub model_download_hint: Option<&'static str>,
    /// Compile-time feature required for builtin mode (`embedding-local`, `stt-local`, …).
    pub required_feature: Option<&'static str>,
    /// Only usable from the macOS desktop app (Tauri + native helper).
    pub desktop_only: bool,
}

pub const LOCAL_MEDIA_PRESETS: &[LocalMediaPreset] = &[
    LocalMediaPreset {
        id: "local-fastembed-minilm",
        label: "FastEmbed MiniLM (on-device)",
        description: "Lightweight ONNX embeddings (~23MB on first use). No API key.",
        capabilities: &[ModelCapability::Embedding],
        mode: LocalMode::Builtin,
        provider: "local_fastembed",
        model: "AllMiniLML6V2",
        base_url: None,
        api_key: Some("local"),
        voice: None,
        docs_url: None,
        model_download_hint: Some(
            "~/.cache/fastembed or memory.pipeline.embedding_local_cache_dir",
        ),
        required_feature: Some("embedding-local"),
        desktop_only: false,
    },
    LocalMediaPreset {
        id: "local-fastembed-bge-zh",
        label: "FastEmbed BGE-small-zh (on-device)",
        description: "Chinese-friendly local embeddings (~30MB on first use).",
        capabilities: &[ModelCapability::Embedding],
        mode: LocalMode::Builtin,
        provider: "local_fastembed",
        model: "BGESmallZHV15",
        base_url: None,
        api_key: Some("local"),
        voice: None,
        docs_url: None,
        model_download_hint: Some("~/.cache/fastembed"),
        required_feature: Some("embedding-local"),
        desktop_only: false,
    },
    LocalMediaPreset {
        id: "ollama-nomic-embed",
        label: "Ollama nomic-embed-text",
        description: "Local embeddings via Ollama (`ollama pull nomic-embed-text`).",
        capabilities: &[ModelCapability::Embedding],
        mode: LocalMode::External,
        provider: "ollama",
        model: "nomic-embed-text",
        base_url: Some("http://127.0.0.1:11434/v1"),
        api_key: Some("ollama"),
        voice: None,
        docs_url: Some("https://ollama.com/library/nomic-embed-text"),
        model_download_hint: Some("ollama pull nomic-embed-text"),
        required_feature: None,
        desktop_only: false,
    },
    LocalMediaPreset {
        id: "ollama-llava-vision",
        label: "Ollama LLaVA (chat + vision)",
        description: "Multimodal chat; images ride on the same model (`ollama pull llava`).",
        capabilities: &[ModelCapability::Chat, ModelCapability::Vision],
        mode: LocalMode::External,
        provider: "ollama",
        model: "llava",
        base_url: Some("http://127.0.0.1:11434/v1/chat/completions"),
        api_key: Some("ollama"),
        voice: None,
        docs_url: Some("https://ollama.com/library/llava"),
        model_download_hint: Some("ollama pull llava"),
        required_feature: None,
        desktop_only: false,
    },
    LocalMediaPreset {
        id: "whisper-cpp-tiny",
        label: "whisper.cpp server (tiny)",
        description: "Local STT via OpenAI-compatible whisper.cpp HTTP server.",
        capabilities: &[ModelCapability::Stt],
        mode: LocalMode::External,
        provider: "whisper_cpp",
        model: "tiny",
        base_url: Some("http://127.0.0.1:8080/v1"),
        api_key: Some("local"),
        voice: None,
        docs_url: Some("https://github.com/ggml-org/whisper.cpp"),
        model_download_hint: Some("Download ggml-tiny.bin and run whisper.cpp server"),
        required_feature: None,
        desktop_only: false,
    },
    LocalMediaPreset {
        id: "apple-speech-macos",
        label: "Apple Speech (macOS native)",
        description: "On-device speech recognition via Apple Speech framework. No model download; macOS desktop app only.",
        capabilities: &[ModelCapability::Stt],
        mode: LocalMode::PlatformNative,
        provider: "apple_speech",
        model: "on-device",
        base_url: None,
        api_key: Some("local"),
        voice: None,
        docs_url: Some("https://developer.apple.com/documentation/speech"),
        model_download_hint: None,
        required_feature: None,
        desktop_only: true,
    },
    LocalMediaPreset {
        id: "local-whisper-tiny",
        label: "whisper.cpp tiny (built-in)",
        description: "On-device STT via whisper.cpp bindings (~39MB model on first use).",
        capabilities: &[ModelCapability::Stt],
        mode: LocalMode::Builtin,
        provider: "local_whisper",
        model: "tiny",
        base_url: None,
        api_key: Some("local"),
        voice: None,
        docs_url: Some("https://github.com/ggml-org/whisper.cpp"),
        model_download_hint: Some("~/.anycode/models/whisper/tiny.bin"),
        required_feature: Some("stt-local"),
        desktop_only: false,
    },
    LocalMediaPreset {
        id: "piper-zh-medium",
        label: "Piper 中文 (huayan-medium)",
        description: "Local TTS via Piper OpenAI-compatible HTTP server.",
        capabilities: &[ModelCapability::Tts],
        mode: LocalMode::External,
        provider: "piper",
        model: "zh_CN-huayan-medium",
        base_url: Some("http://127.0.0.1:5000/v1"),
        api_key: Some("local"),
        voice: Some("zh_CN-huayan-medium"),
        docs_url: Some("https://github.com/rhasspy/piper"),
        model_download_hint: Some("Download voice from huggingface.co/rhasspy/piper-voices"),
        required_feature: None,
        desktop_only: false,
    },
    LocalMediaPreset {
        id: "local-piper-zh",
        label: "Piper 中文 (built-in)",
        description: "On-device TTS via piper-rs (~15–40MB voice on first use).",
        capabilities: &[ModelCapability::Tts],
        mode: LocalMode::Builtin,
        provider: "local_piper",
        model: "zh_CN-huayan-medium",
        base_url: None,
        api_key: Some("local"),
        voice: Some("zh_CN-huayan-medium"),
        docs_url: Some("https://github.com/thewh1teagle/piper-rs"),
        model_download_hint: Some("~/.anycode/models/piper/zh_CN-huayan-medium"),
        required_feature: Some("tts-local"),
        desktop_only: false,
    },
];

/// Preset ids for the recommended lightweight local bundle (external-first).
pub const LIGHTWEIGHT_LOCAL_BUNDLE: &[&str] = &[
    "local-fastembed-minilm",
    "ollama-llava-vision",
    "whisper-cpp-tiny",
    "piper-zh-medium",
];

pub fn preset_by_id(id: &str) -> Option<&'static LocalMediaPreset> {
    LOCAL_MEDIA_PRESETS.iter().find(|p| p.id == id.trim())
}

pub fn presets_for_capability(cap: ModelCapability) -> Vec<&'static LocalMediaPreset> {
    LOCAL_MEDIA_PRESETS
        .iter()
        .filter(|p| p.capabilities.contains(&cap))
        .collect()
}

/// Whether this provider may run without a real API key.
pub fn local_media_provider_allows_placeholder_key(provider: &str) -> bool {
    matches!(
        provider.trim().to_ascii_lowercase().as_str(),
        "local_fastembed"
            | "local_whisper"
            | "local_piper"
            | "apple_speech"
            | "ollama"
            | "whisper_cpp"
            | "piper"
    )
}

pub fn is_builtin_local_provider(provider: &str) -> bool {
    matches!(
        provider.trim().to_ascii_lowercase().as_str(),
        "local_fastembed" | "local_whisper" | "local_piper"
    )
}

/// Convert a preset into a registry item (caller enables capabilities separately).
pub fn preset_to_configured_model(preset: &LocalMediaPreset) -> ConfiguredModelFile {
    let mut extra_headers = None;
    if let Some(voice) = preset.voice {
        let mut map = std::collections::HashMap::new();
        map.insert("voice".to_string(), voice.to_string());
        extra_headers = Some(map);
    }
    ConfiguredModelFile {
        id: preset.id.to_string(),
        display_name: Some(preset.label.to_string()),
        provider: preset.provider.to_string(),
        model: preset.model.to_string(),
        capabilities: preset.capabilities.to_vec(),
        api_key: preset.api_key.map(str::to_string),
        api_key_ref: None,
        plan: None,
        base_url: preset.base_url.map(str::to_string),
        temperature: None,
        max_tokens: None,
        extra_headers,
        endpoint_overrides: None,
        enabled: true,
        tags: Some(vec!["local".to_string()]),
        source: Some("local_preset".to_string()),
    }
}

pub fn local_presets_json() -> Value {
    let presets: Vec<Value> = LOCAL_MEDIA_PRESETS
        .iter()
        .map(|p| {
            let caps: Vec<&str> = p.capabilities.iter().map(|c| c.as_str()).collect();
            let feature_available = p.required_feature.map(feature_available).unwrap_or(true);
            json!({
                "id": p.id,
                "label": p.label,
                "description": p.description,
                "capabilities": caps,
                "mode": match p.mode {
                    LocalMode::Builtin => "builtin",
                    LocalMode::External => "external",
                    LocalMode::PlatformNative => "platform_native",
                },
                "provider": p.provider,
                "model": p.model,
                "base_url": p.base_url,
                "voice": p.voice,
                "docs_url": p.docs_url,
                "model_download_hint": p.model_download_hint,
                "required_feature": p.required_feature,
                "feature_available": feature_available,
                "desktop_only": p.desktop_only,
            })
        })
        .collect();

    json!({
        "presets": presets,
        "lightweight_bundle": LIGHTWEIGHT_LOCAL_BUNDLE,
        "build_features": build_features_json(),
    })
}

fn feature_available(feature: &str) -> bool {
    match feature {
        "embedding-local" => cfg!(feature = "embedding-local"),
        "stt-local" => cfg!(feature = "stt-local"),
        "tts-local" => cfg!(feature = "tts-local"),
        _ => false,
    }
}

pub fn build_features_json() -> Value {
    json!({
        "embedding_local": cfg!(feature = "embedding-local"),
        "stt_local": cfg!(feature = "stt-local"),
        "tts_local": cfg!(feature = "tts-local"),
        "media_local": cfg!(all(feature = "embedding-local", feature = "stt-local", feature = "tts-local")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_presets_have_unique_ids() {
        let mut seen = std::collections::HashSet::new();
        for p in LOCAL_MEDIA_PRESETS {
            assert!(seen.insert(p.id), "duplicate preset id: {}", p.id);
        }
    }

    #[test]
    fn lightweight_bundle_presets_exist() {
        for id in LIGHTWEIGHT_LOCAL_BUNDLE {
            assert!(preset_by_id(id).is_some(), "missing bundle preset: {id}");
        }
    }

    #[test]
    fn preset_to_model_roundtrip() {
        let p = preset_by_id("local-fastembed-minilm").expect("preset");
        let m = preset_to_configured_model(p);
        assert_eq!(m.provider, "local_fastembed");
        assert!(m.capabilities.contains(&ModelCapability::Embedding));
    }
}
