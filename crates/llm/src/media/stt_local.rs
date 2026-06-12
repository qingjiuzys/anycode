//! On-device speech-to-text via whisper.cpp bindings (feature `stt-local`).

use crate::model_cache::whisper_model_path;
use anycode_core::CoreError;

#[cfg(feature = "stt-local")]
use whisper_cpp_plus::WhisperContext;

/// Transcribe 16 kHz mono PCM `f32` samples (whisper.cpp input format).
#[cfg(feature = "stt-local")]
pub async fn transcribe_pcm(model_id: &str, samples: &[f32]) -> Result<String, CoreError> {
    let path = whisper_model_path(model_id);
    if !path.exists() {
        return Err(CoreError::LLMError(format!(
            "whisper model not found at {} — download ggml-{}.bin to this path",
            path.display(),
            model_id
        )));
    }
    let path_str = path.to_string_lossy().into_owned();
    let audio = samples.to_vec();
    let text = tokio::task::spawn_blocking(move || transcribe_blocking(&path_str, &audio))
        .await
        .map_err(|e| CoreError::LLMError(format!("stt join: {e}")))?;
    text
}

#[cfg(feature = "stt-local")]
fn transcribe_blocking(model_path: &str, samples: &[f32]) -> Result<String, CoreError> {
    let ctx = WhisperContext::new(model_path)
        .map_err(|e| CoreError::LLMError(format!("whisper init: {e}")))?;
    ctx.transcribe(samples)
        .map_err(|e| CoreError::LLMError(format!("whisper transcribe: {e}")))
}

#[cfg(not(feature = "stt-local"))]
pub async fn transcribe_pcm(_model_id: &str, _samples: &[f32]) -> Result<String, CoreError> {
    Err(CoreError::LLMError(
        "built-in STT requires build with --features stt-local (or media-local)".into(),
    ))
}

/// Decode WAV bytes to mono f32 @ 16 kHz (simplified: supports 16-bit PCM WAV).
pub fn wav_bytes_to_pcm16k(audio_bytes: &[u8]) -> Result<Vec<f32>, CoreError> {
    if audio_bytes.len() < 44 {
        return Err(CoreError::LLMError("audio too short for WAV".into()));
    }
    if &audio_bytes[0..4] != b"RIFF" || &audio_bytes[8..12] != b"WAVE" {
        return Err(CoreError::LLMError(
            "expected WAV input for local STT".into(),
        ));
    }
    let channels = u16::from_le_bytes([audio_bytes[22], audio_bytes[23]]) as usize;
    let sample_rate = u32::from_le_bytes([
        audio_bytes[24],
        audio_bytes[25],
        audio_bytes[26],
        audio_bytes[27],
    ]) as usize;
    let bits = u16::from_le_bytes([audio_bytes[34], audio_bytes[35]]);
    if bits != 16 {
        return Err(CoreError::LLMError(format!(
            "local STT supports 16-bit PCM WAV only (got {bits}-bit)"
        )));
    }
    let data_start = audio_bytes
        .windows(4)
        .position(|w| w == b"data")
        .map(|i| i + 8)
        .unwrap_or(44);
    let mut samples = Vec::new();
    let bytes = &audio_bytes[data_start..];
    for chunk in bytes.chunks_exact(2 * channels) {
        let l = i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32768.0;
        samples.push(l);
    }
    if sample_rate == 16_000 {
        return Ok(samples);
    }
    resample_linear(&samples, sample_rate, 16_000)
}

fn resample_linear(input: &[f32], from_hz: usize, to_hz: usize) -> Result<Vec<f32>, CoreError> {
    if from_hz == 0 || to_hz == 0 {
        return Err(CoreError::LLMError("invalid sample rate".into()));
    }
    if input.is_empty() {
        return Ok(vec![]);
    }
    let out_len = (input.len() * to_hz) / from_hz;
    let mut out = Vec::with_capacity(out_len.max(1));
    for i in 0..out_len {
        let src = (i as f64 * from_hz as f64) / to_hz as f64;
        let idx = src.floor() as usize;
        let frac = (src - idx as f64) as f32;
        let a = input[idx.min(input.len() - 1)];
        let b = input[(idx + 1).min(input.len() - 1)];
        out.push(a + (b - a) * frac);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_wav_silence_1s() -> Vec<u8> {
        let sample_rate = 16_000u32;
        let channels = 1u16;
        let bits = 16u16;
        let data_bytes = (sample_rate * 2) as usize;
        let mut wav = Vec::new();
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
        wav.extend(std::iter::repeat(0u8).take(data_bytes));
        wav
    }

    #[test]
    fn wav_decode_produces_samples() {
        let wav = minimal_wav_silence_1s();
        let pcm = wav_bytes_to_pcm16k(&wav).expect("decode");
        assert_eq!(pcm.len(), 16_000);
    }
}
