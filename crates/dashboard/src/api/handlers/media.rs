//! Media API: STT status and browser voice transcription for dashboard composers.

use super::*;
use anycode_llm::{
    is_builtin_local_provider,
    media::{MediaClientRegistry, SttClient},
};
use axum::extract::Multipart;

const MAX_AUDIO_BYTES: usize = 10 * 1024 * 1024;

pub async fn get_media_status() -> impl IntoResponse {
    let (_, cfg) = match crate::config_patch::read_config_value(None) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };
    let reg = MediaClientRegistry::from_config(&cfg);
    match reg.stt.as_ref() {
        Some(stt) => Json(json!({
            "stt_configured": true,
            "stt_provider": stt.profile.provider,
            "stt_model": stt.profile.model,
            "stt_builtin": is_builtin_local_provider(&stt.profile.provider),
        }))
        .into_response(),
        None => Json(json!({
            "stt_configured": false,
            "stt_provider": null,
            "stt_model": null,
            "stt_builtin": false,
        }))
        .into_response(),
    }
}

fn is_wav(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE"
}

pub async fn transcribe_audio(mut multipart: Multipart) -> impl IntoResponse {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename = "recording.webm".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "filename" {
            if let Ok(text) = field.text().await {
                let t = text.trim();
                if !t.is_empty() {
                    filename = t.to_string();
                }
            }
            continue;
        }
        if name == "file" {
            match field.bytes().await {
                Ok(bytes) => file_bytes = Some(bytes.to_vec()),
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "ok": false, "error": format!("read upload: {e}") })),
                    )
                        .into_response();
                }
            }
        }
    }

    let audio = match file_bytes {
        Some(b) if !b.is_empty() => b,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "ok": false, "error": "missing audio file field" })),
            )
                .into_response();
        }
    };

    if audio.len() > MAX_AUDIO_BYTES {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": format!("audio too large (max {} MB)", MAX_AUDIO_BYTES / 1024 / 1024)
            })),
        )
            .into_response();
    }

    let (_, cfg) = match crate::config_patch::read_config_value(None) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "ok": false, "error": e.to_string() })),
            )
                .into_response();
        }
    };
    let reg = MediaClientRegistry::from_config(&cfg);
    let stt = match reg.stt.as_ref() {
        Some(s) => s,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "ok": false,
                    "error": "STT not configured — enable a speech-to-text model in Settings → Model & routing"
                })),
            )
                .into_response();
        }
    };

    if stt.profile.provider.eq_ignore_ascii_case("apple_speech") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": "Apple Speech STT runs in the macOS desktop app — open anyCode.app and use the microphone button there"
            })),
        )
            .into_response();
    }

    if is_builtin_local_provider(&stt.profile.provider) && !is_wav(&audio) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": "built-in whisper STT requires 16kHz mono WAV — use an external whisper.cpp preset or let the browser convert before upload"
            })),
        )
            .into_response();
    }

    let client = SttClient::new(stt.profile.clone());
    match client.transcribe(&audio, &filename).await {
        Ok(result) => Json(json!({
            "ok": true,
            "text": result.text,
            "provider": stt.profile.provider,
            "model": stt.profile.model,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": e.to_string(),
                "provider": stt.profile.provider,
            })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_wav_header() {
        assert!(is_wav(b"RIFFxxxxWAVEfmt "));
        assert!(!is_wav(b"not wav"));
    }
}
