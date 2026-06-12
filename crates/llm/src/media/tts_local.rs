//! On-device text-to-speech via piper-rs (feature `tts-local`).

use crate::model_cache::piper_voice_dir;
use anycode_core::CoreError;

#[cfg(feature = "tts-local")]
pub async fn synthesize_local(voice_id: &str, text: &str) -> Result<Vec<u8>, CoreError> {
    let voice_dir = piper_voice_dir(voice_id);
    let onnx = voice_dir.join(format!("{voice_id}.onnx"));
    let json = voice_dir.join(format!("{voice_id}.onnx.json"));
    if !onnx.exists() || !json.exists() {
        return Err(CoreError::LLMError(format!(
            "piper voice not found under {} — download {voice_id}.onnx and .onnx.json",
            voice_dir.display()
        )));
    }
    let text = text.to_string();
    let onnx_str = onnx.to_string_lossy().into_owned();
    let json_str = json.to_string_lossy().into_owned();
    tokio::task::spawn_blocking(move || synthesize_blocking(&onnx_str, &json_str, &text))
        .await
        .map_err(|e| CoreError::LLMError(format!("tts join: {e}")))?
}

#[cfg(feature = "tts-local")]
fn synthesize_blocking(onnx: &str, config: &str, text: &str) -> Result<Vec<u8>, CoreError> {
    use piper_rs::Piper;
    let piper = Piper::from_config(onnx, config)
        .map_err(|e| CoreError::LLMError(format!("piper init: {e}")))?;
    let audio = piper
        .synthesize(text)
        .map_err(|e| CoreError::LLMError(format!("piper synthesize: {e}")))?;
    Ok(audio)
}

#[cfg(not(feature = "tts-local"))]
pub async fn synthesize_local(_voice_id: &str, _text: &str) -> Result<Vec<u8>, CoreError> {
    Err(CoreError::LLMError(
        "built-in TTS requires build with --features tts-local (or media-local)".into(),
    ))
}
