//! Multimodal model clients (STT/TTS/image/video/embedding).

pub mod apple_media;
mod embedding;
mod http;
mod image;
mod registry;
mod stt;
mod stt_local;
mod tts;
mod tts_local;
mod video;

pub mod probe_fixtures;

pub use embedding::EmbeddingClient;
pub use image::{ImageGenClient, ImageGenResult};
pub use registry::{MediaClientRegistry, MediaProfile, ResolvedMediaProfile};
pub use stt::{SttClient, SttResult};
pub use tts::{TtsClient, TtsResult};
pub use video::{VideoGenClient, VideoGenResult};
