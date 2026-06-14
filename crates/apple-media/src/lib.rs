//! macOS native STT/OCR/TTS and desktop utilities via the `anycode-apple-media` Swift helper.
//!
//! ## Architecture seams
//! - **Swift helper** (`apps/anycode-desktop/native/anycode-apple-media/`): Vision, Speech, AVFoundation, etc.
//! - **This crate**: JSON stdin/stdout IPC, helper resolution, temp files — shared by CLI, `anycode-llm`, dashboard.
//! - **Tauri** (`apps/anycode-desktop/src/apple_media.rs`): base64 UI boundary + resource-path helper lookup.
//! - **WeChat** (`crates/cli/src/channels/wx/`): inbound image OCR + voice STT via this crate / `SttClient`.
//! - **Media registry** (`crates/llm/src/media/`): `apple_speech` / `apple_tts` providers route here on macOS.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppleMediaCapabilities {
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

#[derive(Debug, Deserialize)]
struct HelperResponse {
    ok: bool,
    text: Option<String>,
    error: Option<String>,
    data_base64: Option<String>,
    capabilities: Option<AppleMediaCapabilities>,
}

/// Resolve the Apple media helper binary on macOS.
#[cfg(target_os = "macos")]
pub fn resolve_apple_media_helper(extra_paths: &[PathBuf]) -> Option<PathBuf> {
    if let Ok(p) = std::env::var("ANYCODE_APPLE_MEDIA_HELPER") {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }
    for path in extra_paths {
        if path.is_file() {
            return Some(path.clone());
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("anycode-apple-media");
            if sibling.is_file() {
                return Some(sibling);
            }
        }
    }
    if let Some(home) = dirs::home_dir() {
        let installed = home.join(".anycode/bin/anycode-apple-media");
        if installed.is_file() {
            return Some(installed);
        }
    }
    None
}

#[cfg(not(target_os = "macos"))]
pub fn resolve_apple_media_helper(_extra_paths: &[PathBuf]) -> Option<PathBuf> {
    None
}

#[cfg(target_os = "macos")]
pub fn apple_media_available(extra_paths: &[PathBuf]) -> bool {
    resolve_apple_media_helper(extra_paths).is_some()
}

#[cfg(not(target_os = "macos"))]
pub fn apple_media_available(_extra_paths: &[PathBuf]) -> bool {
    false
}

#[cfg(target_os = "macos")]
fn run_helper(helper: &Path, request: serde_json::Value) -> Result<HelperResponse, String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

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
            output.status,
            stderr.trim()
        ));
    }
    Err(format!("parse helper output: raw={line}"))
}

#[cfg(target_os = "macos")]
fn write_temp_file(prefix: &str, ext: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("anycode-apple-media");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!(
        "{prefix}-{}-{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
        ext
    ));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path)
}

#[cfg(target_os = "macos")]
pub fn mime_to_ext(mime: &str) -> &str {
    let m = mime.to_ascii_lowercase();
    if m.contains("png") {
        "png"
    } else if m.contains("jpeg") || m.contains("jpg") {
        "jpg"
    } else if m.contains("webp") {
        "webp"
    } else if m.contains("gif") {
        "gif"
    } else if m.contains("wav") {
        "wav"
    } else if m.contains("amr") {
        "amr"
    } else if m.contains("mp4") || m.contains("m4a") {
        "m4a"
    } else if m.contains("webm") {
        "webm"
    } else if m.contains("mpeg") || m.contains("mp3") {
        "mp3"
    } else {
        "bin"
    }
}

#[cfg(target_os = "macos")]
fn filename_to_ext(filename: &str) -> &str {
    Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin")
}

/// Probe native media capabilities and permission state.
#[cfg(target_os = "macos")]
pub fn query_capabilities(extra_paths: &[PathBuf]) -> Option<AppleMediaCapabilities> {
    let helper = resolve_apple_media_helper(extra_paths)?;
    let resp = run_helper(&helper, serde_json::json!({ "op": "capabilities" })).ok()?;
    if resp.ok {
        resp.capabilities.or_else(|| {
            Some(AppleMediaCapabilities {
                stt: true,
                ocr: true,
                tts: true,
                notify: true,
                keychain: true,
                pasteboard: true,
                platform: "macos".into(),
                helper_path: Some(helper.display().to_string()),
                speech_authorized: None,
                microphone_authorized: None,
            })
        })
    } else {
        None
    }
}

#[cfg(not(target_os = "macos"))]
pub fn query_capabilities(_extra_paths: &[PathBuf]) -> Option<AppleMediaCapabilities> {
    None
}

/// Run on-device OCR via Apple Vision.
#[cfg(target_os = "macos")]
pub fn ocr_image_bytes(
    extra_paths: &[PathBuf],
    mime: &str,
    bytes: &[u8],
    languages: Option<&[String]>,
) -> Option<String> {
    let helper = resolve_apple_media_helper(extra_paths)?;
    let ext = mime_to_ext(mime);
    let path = write_temp_file("ocr", ext, bytes).ok()?;
    let langs = languages
        .map(|l| l.to_vec())
        .unwrap_or_else(|| vec!["zh-Hans".into(), "en-US".into()]);
    let resp = run_helper(
        &helper,
        serde_json::json!({
            "op": "ocr",
            "image_path": path.display().to_string(),
            "languages": langs,
        }),
    )
    .ok()?;
    let _ = std::fs::remove_file(&path);
    if resp.ok {
        resp.text.filter(|t| !t.trim().is_empty())
    } else {
        tracing::debug!(
            error = resp.error.as_deref().unwrap_or("ocr failed"),
            "apple media OCR failed"
        );
        None
    }
}

#[cfg(not(target_os = "macos"))]
pub fn ocr_image_bytes(
    _extra_paths: &[PathBuf],
    _mime: &str,
    _bytes: &[u8],
    _languages: Option<&[String]>,
) -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
fn is_wav(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE"
}

/// Convert arbitrary audio bytes to 16 kHz mono WAV via AVFoundation.
#[cfg(target_os = "macos")]
pub fn convert_audio_to_wav(
    extra_paths: &[PathBuf],
    audio_bytes: &[u8],
    filename: &str,
) -> Result<Vec<u8>, String> {
    if is_wav(audio_bytes) {
        return Ok(audio_bytes.to_vec());
    }
    let helper = resolve_apple_media_helper(extra_paths);
    if let Some(helper) = helper {
        let ext = filename_to_ext(filename);
        let in_path = write_temp_file("convert-in", ext, audio_bytes)?;
        let out_path = write_temp_file("convert-out", "wav", &[])?;
        let _ = std::fs::remove_file(&out_path);
        if let Ok(resp) = run_helper(
            &helper,
            serde_json::json!({
                "op": "convert",
                "input_path": in_path.display().to_string(),
                "output_path": out_path.display().to_string(),
                "format": "wav",
            }),
        ) {
            let _ = std::fs::remove_file(&in_path);
            if resp.ok {
                return std::fs::read(&out_path).map_err(|e| e.to_string());
            }
        } else {
            let _ = std::fs::remove_file(&in_path);
        }
    }
    convert_audio_with_afconvert(audio_bytes, filename)
}

#[cfg(target_os = "macos")]
fn convert_audio_with_afconvert(audio_bytes: &[u8], filename: &str) -> Result<Vec<u8>, String> {
    use std::process::Command;

    let ext = filename_to_ext(filename);
    let in_path = write_temp_file("afconvert-in", ext, audio_bytes)?;
    let out_path = write_temp_file("afconvert-out", "wav", &[])?;
    let _ = std::fs::remove_file(&out_path);
    let output = Command::new("/usr/bin/afconvert")
        .args([
            "-f",
            "WAVE",
            "-d",
            "LEI16@16000",
            in_path.to_str().unwrap_or(""),
            out_path.to_str().unwrap_or(""),
        ])
        .output()
        .map_err(|e| format!("afconvert spawn: {e}"))?;
    let _ = std::fs::remove_file(&in_path);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("afconvert failed: {}", stderr.trim()));
    }
    std::fs::read(&out_path).map_err(|e| e.to_string())
}

#[cfg(not(target_os = "macos"))]
pub fn convert_audio_to_wav(
    _extra_paths: &[PathBuf],
    _audio_bytes: &[u8],
    _filename: &str,
) -> Result<Vec<u8>, String> {
    Err("audio convert requires macOS".into())
}

/// Transcribe audio via Apple Speech (`SFSpeechRecognizer`).
#[cfg(target_os = "macos")]
pub fn transcribe_audio_bytes(
    extra_paths: &[PathBuf],
    audio_bytes: &[u8],
    filename: &str,
    locale: &str,
) -> Result<String, String> {
    let helper = resolve_apple_media_helper(extra_paths)
        .ok_or_else(|| "anycode-apple-media helper not found".to_string())?;

    let (bytes, ext) = if is_wav(audio_bytes) {
        (audio_bytes.to_vec(), "wav".to_string())
    } else {
        let wav = convert_audio_to_wav(extra_paths, audio_bytes, filename)?;
        (wav, "wav".to_string())
    };

    let path = write_temp_file("stt", &ext, &bytes)?;
    let resp = run_helper(
        &helper,
        serde_json::json!({
            "op": "stt",
            "audio_path": path.display().to_string(),
            "locale": locale,
        }),
    )?;
    let _ = std::fs::remove_file(&path);
    if resp.ok {
        resp.text
            .filter(|t| !t.trim().is_empty())
            .ok_or_else(|| "empty transcription".to_string())
    } else {
        Err(resp
            .error
            .unwrap_or_else(|| "Apple Speech STT failed".to_string()))
    }
}

#[cfg(not(target_os = "macos"))]
pub fn transcribe_audio_bytes(
    _extra_paths: &[PathBuf],
    _audio_bytes: &[u8],
    _filename: &str,
    _locale: &str,
) -> Result<String, String> {
    Err("Apple Speech STT requires macOS".into())
}

/// Synthesize speech via `AVSpeechSynthesizer`.
#[cfg(target_os = "macos")]
pub fn synthesize_speech(
    extra_paths: &[PathBuf],
    text: &str,
    voice: Option<&str>,
    locale: &str,
) -> Result<Vec<u8>, String> {
    let helper = resolve_apple_media_helper(extra_paths)
        .ok_or_else(|| "anycode-apple-media helper not found".to_string())?;
    let out_path = write_temp_file("tts-out", "wav", &[])?;
    let _ = std::fs::remove_file(&out_path);
    let resp = run_helper(
        &helper,
        serde_json::json!({
            "op": "tts",
            "text": text,
            "voice": voice,
            "locale": locale,
            "output_path": out_path.display().to_string(),
        }),
    )?;
    if !resp.ok {
        return Err(resp.error.unwrap_or_else(|| "Apple TTS failed".to_string()));
    }
    if let Some(b64) = resp.data_base64 {
        use base64::Engine;
        return base64::engine::general_purpose::STANDARD
            .decode(b64.trim())
            .map_err(|e| format!("decode tts audio: {e}"));
    }
    std::fs::read(&out_path).map_err(|e| e.to_string())
}

#[cfg(not(target_os = "macos"))]
pub fn synthesize_speech(
    _extra_paths: &[PathBuf],
    _text: &str,
    _voice: Option<&str>,
    _locale: &str,
) -> Result<Vec<u8>, String> {
    Err("Apple TTS requires macOS".into())
}

/// Post a macOS user notification.
#[cfg(target_os = "macos")]
pub fn post_notification(extra_paths: &[PathBuf], title: &str, body: &str) -> Result<(), String> {
    let helper = resolve_apple_media_helper(extra_paths)
        .ok_or_else(|| "anycode-apple-media helper not found".to_string())?;
    let resp = run_helper(
        &helper,
        serde_json::json!({
            "op": "notify",
            "title": title,
            "body": body,
        }),
    )?;
    if resp.ok {
        Ok(())
    } else {
        Err(resp
            .error
            .unwrap_or_else(|| "notification failed".to_string()))
    }
}

#[cfg(not(target_os = "macos"))]
pub fn post_notification(
    _extra_paths: &[PathBuf],
    _title: &str,
    _body: &str,
) -> Result<(), String> {
    Err("macOS notifications require macOS".into())
}

/// Read a generic password from Keychain.
#[cfg(target_os = "macos")]
pub fn keychain_get(
    extra_paths: &[PathBuf],
    service: &str,
    account: &str,
) -> Result<Option<String>, String> {
    let helper = resolve_apple_media_helper(extra_paths)
        .ok_or_else(|| "anycode-apple-media helper not found".to_string())?;
    let resp = run_helper(
        &helper,
        serde_json::json!({
            "op": "keychain_get",
            "service": service,
            "account": account,
        }),
    )?;
    if resp.ok {
        Ok(resp.text.filter(|t| !t.is_empty()))
    } else {
        Err(resp
            .error
            .unwrap_or_else(|| "keychain read failed".to_string()))
    }
}

#[cfg(not(target_os = "macos"))]
pub fn keychain_get(
    _extra_paths: &[PathBuf],
    _service: &str,
    _account: &str,
) -> Result<Option<String>, String> {
    Err("Keychain requires macOS".into())
}

/// Store a generic password in Keychain.
#[cfg(target_os = "macos")]
pub fn keychain_set(
    extra_paths: &[PathBuf],
    service: &str,
    account: &str,
    secret: &str,
) -> Result<(), String> {
    let helper = resolve_apple_media_helper(extra_paths)
        .ok_or_else(|| "anycode-apple-media helper not found".to_string())?;
    let resp = run_helper(
        &helper,
        serde_json::json!({
            "op": "keychain_set",
            "service": service,
            "account": account,
            "secret": secret,
        }),
    )?;
    if resp.ok {
        Ok(())
    } else {
        Err(resp
            .error
            .unwrap_or_else(|| "keychain write failed".to_string()))
    }
}

#[cfg(not(target_os = "macos"))]
pub fn keychain_set(
    _extra_paths: &[PathBuf],
    _service: &str,
    _account: &str,
    _secret: &str,
) -> Result<(), String> {
    Err("Keychain requires macOS".into())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteboardItem {
    pub kind: String,
    pub mime_type: Option<String>,
    pub text: Option<String>,
    pub data_base64: Option<String>,
}

/// Read the current pasteboard (text, image, file URL).
#[cfg(target_os = "macos")]
pub fn read_pasteboard(extra_paths: &[PathBuf]) -> Result<Vec<PasteboardItem>, String> {
    let helper = resolve_apple_media_helper(extra_paths)
        .ok_or_else(|| "anycode-apple-media helper not found".to_string())?;
    let resp = run_helper(&helper, serde_json::json!({ "op": "pasteboard_read" }))?;
    if !resp.ok {
        return Err(resp
            .error
            .unwrap_or_else(|| "pasteboard read failed".to_string()));
    }
    let text = resp.text.unwrap_or_default();
    if text.is_empty() {
        return Ok(vec![]);
    }
    serde_json::from_str(&text).map_err(|e| format!("parse pasteboard items: {e}"))
}

#[cfg(not(target_os = "macos"))]
pub fn read_pasteboard(_extra_paths: &[PathBuf]) -> Result<Vec<PasteboardItem>, String> {
    Err("Pasteboard requires macOS".into())
}

/// Default empty extra path list for CLI / server callers.
pub const NO_EXTRA_PATHS: &[PathBuf] = &[];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_unavailable_off_macos() {
        #[cfg(not(target_os = "macos"))]
        assert!(!apple_media_available(NO_EXTRA_PATHS));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn mime_to_ext_mapping() {
        assert_eq!(mime_to_ext("image/png"), "png");
        assert_eq!(mime_to_ext("audio/amr"), "amr");
    }
}
