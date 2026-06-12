//! macOS native STT/OCR via bundled `anycode-apple-media` helper.

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize)]
struct HelperResponse {
    ok: bool,
    text: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppleMediaCapabilities {
    pub stt: bool,
    pub ocr: bool,
    pub platform: String,
    pub helper_path: Option<String>,
}

fn resolve_apple_media_helper(app: &AppHandle) -> Option<PathBuf> {
    let candidates = [
        "resources/bin/anycode-apple-media",
        "bin/anycode-apple-media",
        "_up_/resources/bin/anycode-apple-media",
    ];
    for rel in candidates {
        if let Ok(p) = app.path().resolve(rel, tauri::path::BaseDirectory::Resource) {
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

fn run_helper(helper: &Path, request: serde_json::Value) -> Result<HelperResponse, String> {
    let mut child = Command::new(helper)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn {}: {e}", helper.display()))?;

    if let Some(mut stdin) = child.stdin.take() {
        let body = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
        stdin.write_all(&body).map_err(|e| e.to_string())?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("wait helper: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next().unwrap_or("").trim();
    if let Ok(resp) = serde_json::from_str::<HelperResponse>(line) {
        return Ok(resp);
    }
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "anycode-apple-media exited {}: {}",
            output.status, stderr.trim()
        ));
    }
    Err(format!("parse helper output: raw={line}"))
}

fn write_temp_file(prefix: &str, ext: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("anycode-apple-media");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!(
        "{prefix}-{}-{}.{ext}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path)
}

fn mime_to_ext(mime: &str) -> &str {
    if mime.contains("png") {
        "png"
    } else if mime.contains("jpeg") || mime.contains("jpg") {
        "jpg"
    } else if mime.contains("webp") {
        "webp"
    } else if mime.contains("wav") {
        "wav"
    } else if mime.contains("mp4") || mime.contains("m4a") {
        "m4a"
    } else if mime.contains("webm") {
        "webm"
    } else {
        "bin"
    }
}

fn transcribe_blocking(
    helper: PathBuf,
    audio_base64: String,
    mime_type: Option<String>,
    locale: Option<String>,
) -> Result<String, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(audio_base64.trim())
        .map_err(|e| format!("invalid audio_base64: {e}"))?;
    let ext = mime_to_ext(mime_type.as_deref().unwrap_or("audio/wav"));
    let path = write_temp_file("stt", ext, &bytes)?;
    let result = run_helper(
        &helper,
        json!({
            "op": "stt",
            "audio_path": path.display().to_string(),
            "locale": locale.unwrap_or_else(|| "zh-CN".into()),
        }),
    );
    let _ = std::fs::remove_file(&path);
    let resp = result?;
    if resp.ok {
        resp.text
            .filter(|t| !t.trim().is_empty())
            .ok_or_else(|| "empty transcription".to_string())
    } else {
        Err(resp.error.unwrap_or_else(|| "STT failed".to_string()))
    }
}

fn ocr_blocking(
    helper: PathBuf,
    image_base64: String,
    mime_type: Option<String>,
    languages: Option<Vec<String>>,
) -> Result<String, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(image_base64.trim())
        .map_err(|e| format!("invalid image_base64: {e}"))?;
    let ext = mime_to_ext(mime_type.as_deref().unwrap_or("image/png"));
    let path = write_temp_file("ocr", ext, &bytes)?;
    let result = run_helper(
        &helper,
        json!({
            "op": "ocr",
            "image_path": path.display().to_string(),
            "languages": languages.unwrap_or_else(|| vec!["zh-Hans".into(), "en-US".into()]),
        }),
    );
    let _ = std::fs::remove_file(&path);
    let resp = result?;
    if resp.ok {
        resp.text
            .filter(|t| !t.trim().is_empty())
            .ok_or_else(|| "no text recognized".to_string())
    } else {
        Err(resp.error.unwrap_or_else(|| "OCR failed".to_string()))
    }
}

#[tauri::command]
pub async fn apple_media_capabilities(app: AppHandle) -> AppleMediaCapabilities {
    let helper = resolve_apple_media_helper(&app);
    AppleMediaCapabilities {
        stt: helper.is_some(),
        ocr: helper.is_some(),
        platform: "macos".into(),
        helper_path: helper.as_ref().map(|p| p.display().to_string()),
    }
}

#[tauri::command]
pub async fn apple_media_transcribe(
    app: AppHandle,
    audio_base64: String,
    mime_type: Option<String>,
    locale: Option<String>,
) -> Result<String, String> {
    let helper = resolve_apple_media_helper(&app).ok_or_else(|| {
        "anycode-apple-media helper not found — rebuild desktop app on macOS".to_string()
    })?;
    tauri::async_runtime::spawn_blocking(move || {
        transcribe_blocking(helper, audio_base64, mime_type, locale)
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
    let helper = resolve_apple_media_helper(&app).ok_or_else(|| {
        "anycode-apple-media helper not found — rebuild desktop app on macOS".to_string()
    })?;
    tauri::async_runtime::spawn_blocking(move || {
        ocr_blocking(helper, image_base64, mime_type, languages)
    })
    .await
    .map_err(|e| format!("ocr task failed: {e}"))?
}
