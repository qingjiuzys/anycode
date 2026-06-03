//! Model capability taxonomy (chat vs media modalities).

use serde::{Deserialize, Serialize};

/// What a configured model profile is used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapability {
    Chat,
    /// Multimodal chat input (vision); may share transport with chat.
    Vision,
    Embedding,
    Stt,
    Tts,
    ImageGen,
    VideoGen,
    /// Reserved for rerank APIs.
    Rerank,
}

impl ModelCapability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Vision => "vision",
            Self::Embedding => "embedding",
            Self::Stt => "stt",
            Self::Tts => "tts",
            Self::ImageGen => "image",
            Self::VideoGen => "video",
            Self::Rerank => "rerank",
        }
    }

    pub fn all() -> &'static [ModelCapability] {
        &[
            Self::Chat,
            Self::Vision,
            Self::Embedding,
            Self::Stt,
            Self::Tts,
            Self::ImageGen,
            Self::VideoGen,
        ]
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "chat" => Some(Self::Chat),
            "vision" | "multimodal" => Some(Self::Vision),
            "embedding" | "embed" => Some(Self::Embedding),
            "stt" | "speech_to_text" | "transcription" => Some(Self::Stt),
            "tts" | "text_to_speech" => Some(Self::Tts),
            "speech" => Some(Self::Stt),
            "image" | "image_gen" | "imagegen" => Some(Self::ImageGen),
            "video" | "video_gen" | "videogen" => Some(Self::VideoGen),
            "rerank" => Some(Self::Rerank),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_capability_aliases() {
        assert_eq!(ModelCapability::parse("stt"), Some(ModelCapability::Stt));
        assert_eq!(
            ModelCapability::parse("image_gen"),
            Some(ModelCapability::ImageGen)
        );
    }
}
