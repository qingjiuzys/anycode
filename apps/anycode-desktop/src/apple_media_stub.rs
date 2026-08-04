//! Non-macOS stubs so the same invoke handlers exist when cross-compiling.

use serde::Serialize;
use tauri::AppHandle;

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

#[tauri::command]
pub async fn apple_media_capabilities(_app: AppHandle) -> AppleMediaCapabilitiesView {
    AppleMediaCapabilitiesView {
        stt: false,
        ocr: false,
        tts: false,
        notify: false,
        keychain: false,
        pasteboard: false,
        platform: std::env::consts::OS.into(),
        helper_path: None,
        speech_authorized: None,
        microphone_authorized: None,
    }
}

#[tauri::command]
pub async fn apple_media_transcribe(
    _app: AppHandle,
    _audio_base64: String,
    _mime_type: Option<String>,
    _locale: Option<String>,
) -> Result<String, String> {
    Err("apple media is only available on macOS".into())
}

#[tauri::command]
pub async fn apple_media_ocr_image(
    _app: AppHandle,
    _image_base64: String,
    _mime_type: Option<String>,
    _languages: Option<Vec<String>>,
) -> Result<String, String> {
    Err("apple media is only available on macOS".into())
}

#[tauri::command]
pub async fn apple_media_synthesize(
    _app: AppHandle,
    _text: String,
    _voice: Option<String>,
    _locale: Option<String>,
) -> Result<String, String> {
    Err("apple media is only available on macOS".into())
}

#[tauri::command]
pub async fn apple_media_read_pasteboard(
    _app: AppHandle,
) -> Result<Vec<serde_json::Value>, String> {
    Err("apple media is only available on macOS".into())
}

#[tauri::command]
pub async fn apple_media_notify(
    _app: AppHandle,
    _title: String,
    _body: String,
) -> Result<(), String> {
    Err("apple media is only available on macOS".into())
}
