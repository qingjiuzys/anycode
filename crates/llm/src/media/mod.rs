//! Multimodal model clients (STT/TTS/image/video/embedding).

mod embedding;
mod http;
mod image;
mod registry;
mod stt;
mod tts;
mod video;

pub use embedding::EmbeddingClient;
pub use image::{ImageGenClient, ImageGenResult};
pub use registry::{MediaClientRegistry, MediaProfile, ResolvedMediaProfile};
pub use stt::{SttClient, SttResult};
pub use tts::{TtsClient, TtsResult};
pub use video::{VideoGenClient, VideoGenResult};
