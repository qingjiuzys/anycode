//! On-device text-to-speech via piper-rs (feature `tts-local`).

use crate::model_cache::piper_voice_dir;
use anycode_core::CoreError;
use std::path::Path;

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
    tokio::task::spawn_blocking(move || synthesize_blocking(&onnx, &json, &text))
        .await
        .map_err(|e| CoreError::LLMError(format!("tts join: {e}")))?
}

#[cfg(feature = "tts-local")]
fn synthesize_blocking(onnx: &Path, config: &Path, text: &str) -> Result<Vec<u8>, CoreError> {
    use piper_rs::Piper;
    let mut piper =
        Piper::new(onnx, config).map_err(|e| CoreError::LLMError(format!("piper init: {e}")))?;
    let (samples, sample_rate) = piper
        .create(text, false, None, None, None, None)
        .map_err(|e| CoreError::LLMError(format!("piper synthesize: {e}")))?;
    Ok(encode_wav_f32_mono(&samples, sample_rate))
}

#[cfg(feature = "tts-local")]
fn encode_wav_f32_mono(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let channels = 1u16;
    let bits = 16u16;
    let data_bytes = samples.len() * 2;
    let mut wav = Vec::with_capacity(44 + data_bytes);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36u32 + data_bytes as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * channels as u32 * bits as u32 / 8).to_le_bytes());
    wav.extend_from_slice(&(channels * bits / 8).to_le_bytes());
    wav.extend_from_slice(&bits.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_bytes as u32).to_le_bytes());
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let v = if clamped < 0.0 {
            (clamped * 32768.0) as i16
        } else {
            (clamped * 32767.0) as i16
        };
        wav.extend_from_slice(&v.to_le_bytes());
    }
    wav
}

#[cfg(not(feature = "tts-local"))]
pub async fn synthesize_local(_voice_id: &str, _text: &str) -> Result<Vec<u8>, CoreError> {
    Err(CoreError::LLMError(
        "built-in TTS requires build with --features tts-local (or media-local)".into(),
    ))
}
