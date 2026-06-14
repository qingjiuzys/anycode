//! macOS native STT/OCR/TTS via bundled `anycode-apple-media` helper (Tauri boundary).

use anycode_apple_media::{
    ocr_image_bytes, query_capabilities, synthesize_speech, transcribe_audio_bytes,
    AppleMediaCapabilities, PasteboardItem,
};
use base64::Engine;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppleMediaCapabilitiesView {
    pub stt: bool,
    pub ocr: bool,
    pub tts: bool,
    pub notify: bool,
    pub keychain: bool,
    pub pasteboard: bool,
    pub platform: String,
    pub helper_path: Option<String>,
    pub speech_authorized: Option<bool>,
    pub microphone_authorized: Option<bool>,
}

impl From<AppleMediaCapabilities> for AppleMediaCapabilitiesView {
    fn from(c: AppleMediaCapabilities) -> Self {
        Self {
            stt: c.stt,
            ocr: c.ocr,
            tts: c.tts,
            notify: c.notify,
            keychain: c.keychain,
            pasteboard: c.pasteboard,
            platform: c.platform,
            helper_path: c.helper_path,
            speech_authorized: c.speech_authorized,
            microphone_authorized: c.microphone_authorized,
        }
    }
}

fn resolve_extra_paths(app: &AppHandle) -> Vec<PathBuf> {
    let candidates = [
        "resources/bin/anycode-apple-media",
        "bin/anycode-apple-media",
        "_up_/resources/bin/anycode-apple-media",
    ];
    let mut paths = Vec::new();
    for rel in candidates {
        if let Ok(p) = app.path().resolve(rel, tauri::path::BaseDirectory::Resource) {
            if p.is_file() {
                paths.push(p);
            }
        }
    }
    paths
}

fn mime_to_ext(mime: &str) -> &str {
    anycode_apple_media::mime_to_ext(mime)
}

#[tauri::command]
pub async fn apple_media_capabilities(app: AppHandle) -> AppleMediaCapabilitiesView {
    let extra = resolve_extra_paths(&app);
    query_capabilities(&extra).map(Into::into).unwrap_or_else(|| {
        AppleMediaCapabilitiesView {
            stt: false,
            ocr: false,
            tts: false,
            notify: false,
            keychain: false,
            pasteboard: false,
            platform: "macos".into(),
            helper_path: extra.first().map(|p| p.display().to_string()),
            speech_authorized: None,
            microphone_authorized: None,
        }
    })
}

#[tauri::command]
pub async fn apple_media_transcribe(
    app: AppHandle,
    audio_base64: String,
    mime_type: Option<String>,
    locale: Option<String>,
) -> Result<String, String> {
    let extra = resolve_extra_paths(&app);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(audio_base64.trim())
        .map_err(|e| format!("invalid audio_base64: {e}"))?;
    let ext = mime_to_ext(mime_type.as_deref().unwrap_or("audio/wav"));
    let filename = format!("recording.{ext}");
    let locale = locale.unwrap_or_else(|| "zh-CN".into());
    tauri::async_runtime::spawn_blocking(move || {
        transcribe_audio_bytes(&extra, &bytes, &filename, &locale)
    })
    .await
    .map_err(|e| format!("transcribe task failed: {e}"))?
}

#[tauri::command]
pub async fn apple_media_ocr_image(
    app: AppHandle,
    image_base64: String,
    mime_type: Option<String>,
    languages: Option<Vec<String>>,
) -> Result<String, String> {
    let extra = resolve_extra_paths(&app);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(image_base64.trim())
        .map_err(|e| format!("invalid image_base64: {e}"))?;
    let mime = mime_type.unwrap_or_else(|| "image/png".into());
    let langs = languages;
    tauri::async_runtime::spawn_blocking(move || {
        ocr_image_bytes(&extra, &mime, &bytes, langs.as_deref())
            .ok_or_else(|| "no text recognized".to_string())
    })
    .await
    .map_err(|e| format!("ocr task failed: {e}"))?
}

#[tauri::command]
pub async fn apple_media_synthesize(
    app: AppHandle,
    text: String,
    voice: Option<String>,
    locale: Option<String>,
) -> Result<String, String> {
    let extra = resolve_extra_paths(&app);
    let locale = locale.unwrap_or_else(|| "zh-CN".into());
    tauri::async_runtime::spawn_blocking(move || {
        synthesize_speech(&extra, &text, voice.as_deref(), &locale).map(|bytes| {
            base64::engine::general_purpose::STANDARD.encode(bytes)
        })
    })
    .await
    .map_err(|e| format!("tts task failed: {e}"))?
}

#[tauri::command]
pub async fn apple_media_read_pasteboard(app: AppHandle) -> Result<Vec<PasteboardItem>, String> {
    let extra = resolve_extra_paths(&app);
    tauri::async_runtime::spawn_blocking(move || anycode_apple_media::read_pasteboard(&extra))
        .await
        .map_err(|e| format!("pasteboard task failed: {e}"))?
}

#[tauri::command]
pub async fn apple_media_notify(
    app: AppHandle,
    title: String,
    body: String,
) -> Result<(), String> {
    let extra = resolve_extra_paths(&app);
    tauri::async_runtime::spawn_blocking(move || {
        anycode_apple_media::post_notification(&extra, &title, &body)
    })
    .await
    .map_err(|e| format!("notify task failed: {e}"))?
}
