//! Media API: STT/TTS/OCR — capability slots independent of the active chat brain.

use super::*;
use anycode_llm::{
    is_builtin_local_provider,
    media::{apple_media, MediaClientRegistry, SttClient, TtsClient},
};
use axum::extract::Multipart;
use serde::Deserialize;

const MAX_AUDIO_BYTES: usize = 10 * 1024 * 1024;
const MAX_TTS_CHARS: usize = 4_000;

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
    let apple_caps = apple_media::query_capabilities(apple_media::NO_EXTRA_PATHS);
    let ocr_available = crate::control::vision_payload::ocr_fallback_available();
    let chat_vision =
        crate::control::vision_payload::active_chat_supports_vision().unwrap_or(false);
    let image_attach_ok = chat_vision || ocr_available;
    Json(json!({
        "stt_configured": reg.stt.is_some(),
        "stt_provider": reg.stt.as_ref().map(|s| &s.profile.provider),
        "stt_model": reg.stt.as_ref().map(|s| &s.profile.model),
        "stt_builtin": reg.stt.as_ref().map(|s| is_builtin_local_provider(&s.profile.provider)),
        "tts_configured": reg.tts.is_some(),
        "tts_provider": reg.tts.as_ref().map(|s| &s.profile.provider),
        "tts_model": reg.tts.as_ref().map(|s| &s.profile.model),
        "ocr_available": ocr_available,
        "chat_supports_vision": chat_vision,
        "image_attach_ok": image_attach_ok,
        "apple_media": apple_caps,
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
pub struct OcrImageRequest {
    pub mime_type: String,
    pub data_base64: String,
}

#[derive(Debug, Deserialize)]
pub struct OcrRequest {
    pub images: Vec<OcrImageRequest>,
}

/// OCR images via Apple Vision helper — for text-only chat brains.
pub async fn ocr_images(Json(body): Json<OcrRequest>) -> impl IntoResponse {
    let payloads: Vec<crate::control::vision_payload::VisionImagePayload> = body
        .images
        .into_iter()
        .map(|img| crate::control::vision_payload::VisionImagePayload {
            mime_type: img.mime_type,
            data_base64: img.data_base64,
        })
        .collect();
    match tokio::task::spawn_blocking(move || {
        crate::control::vision_payload::ocr_images_to_text(&payloads)
    })
    .await
    {
        Ok(Ok(text)) => {
            Json(json!({ "ok": true, "text": text, "provider": "apple_ocr" })).into_response()
        }
        Ok(Err(e)) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "ok": false, "error": format!("ocr task: {e}") })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct TtsRequest {
    pub text: String,
}

/// Synthesize speech with the active TTS capability model (not the chat brain).
pub async fn synthesize_speech(Json(body): Json<TtsRequest>) -> impl IntoResponse {
    let text = body.text.trim().to_string();
    if text.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": "text is required" })),
        )
            .into_response();
    }
    if text.chars().count() > MAX_TTS_CHARS {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": format!("text too long for TTS (max {MAX_TTS_CHARS} chars)")
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
    let tts = match reg.tts.as_ref() {
        Some(t) => t,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "ok": false,
                    "error": "TTS not configured — enable a text-to-speech model in Settings → Model & routing"
                })),
            )
                .into_response();
        }
    };
    let client = TtsClient::new(tts.profile.clone());
    match client.synthesize(&text).await {
        Ok(result) => {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&result.audio_bytes);
            Json(json!({
                "ok": true,
                "audio_base64": b64,
                "mime_type": result.content_type,
                "provider": tts.profile.provider,
                "model": tts.profile.model,
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": e.to_string(),
                "provider": tts.profile.provider,
            })),
        )
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

    if stt.profile.provider.eq_ignore_ascii_case("apple_speech")
        && !apple_media::apple_media_available()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": "Apple Speech STT requires macOS with anycode-apple-media helper installed"
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
