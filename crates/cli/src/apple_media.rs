//! macOS native STT/OCR via the shared `anycode-apple-media` bridge.

pub use anycode_apple_media::{
    apple_media_available, ocr_image_bytes, query_capabilities, transcribe_audio_bytes,
    AppleMediaCapabilities, NO_EXTRA_PATHS,
};

pub fn apple_media_ocr_available() -> bool {
    apple_media_available(NO_EXTRA_PATHS)
}

pub fn resolve_apple_media_helper() -> Option<std::path::PathBuf> {
    anycode_apple_media::resolve_apple_media_helper(NO_EXTRA_PATHS)
}

pub fn ocr_image(mime: &str, bytes: &[u8]) -> Option<String> {
    ocr_image_bytes(NO_EXTRA_PATHS, mime, bytes, None)
}

pub fn transcribe_audio(
    audio_bytes: &[u8],
    filename: &str,
    locale: &str,
) -> Result<String, String> {
    transcribe_audio_bytes(NO_EXTRA_PATHS, audio_bytes, filename, locale)
}

pub fn notify_approval_pending(tool: &str) -> Result<(), String> {
    anycode_apple_media::post_notification(
        NO_EXTRA_PATHS,
        "anyCode — tool approval",
        &format!("Approve tool: {tool}"),
    )
}
