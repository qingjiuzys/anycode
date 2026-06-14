//! macOS native media bridge — re-export shared crate and LLM-specific helpers.

pub use anycode_apple_media::{
    convert_audio_to_wav, keychain_get, keychain_set, mime_to_ext, ocr_image_bytes,
    post_notification, query_capabilities, read_pasteboard, synthesize_speech,
    transcribe_audio_bytes, AppleMediaCapabilities, PasteboardItem, NO_EXTRA_PATHS,
};

use anycode_core::CoreError;

pub fn is_apple_speech_provider(provider: &str) -> bool {
    provider.eq_ignore_ascii_case("apple_speech")
}

pub fn is_apple_tts_provider(provider: &str) -> bool {
    provider.eq_ignore_ascii_case("apple_tts")
}

pub fn apple_media_available() -> bool {
    anycode_apple_media::apple_media_available(NO_EXTRA_PATHS)
}

pub async fn transcribe_apple_speech(
    audio_bytes: &[u8],
    filename: &str,
    locale: &str,
) -> Result<String, CoreError> {
    tokio::task::spawn_blocking({
        let audio = audio_bytes.to_vec();
        let filename = filename.to_string();
        let locale = locale.to_string();
        move || {
            transcribe_audio_bytes(NO_EXTRA_PATHS, &audio, &filename, &locale)
                .map_err(|e| CoreError::LLMError(e))
        }
    })
    .await
    .map_err(|e| CoreError::LLMError(format!("apple speech task: {e}")))?
}

pub async fn synthesize_apple_tts(
    text: &str,
    voice: Option<&str>,
    locale: &str,
) -> Result<Vec<u8>, CoreError> {
    let text = text.to_string();
    let voice = voice.map(str::to_string);
    let locale = locale.to_string();
    tokio::task::spawn_blocking(move || {
        synthesize_speech(NO_EXTRA_PATHS, &text, voice.as_deref(), &locale)
            .map_err(|e| CoreError::LLMError(e))
    })
    .await
    .map_err(|e| CoreError::LLMError(format!("apple tts task: {e}")))?
}
