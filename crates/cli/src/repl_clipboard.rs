//! 系统剪贴板读取：在 raw TTY 下 `Event::Paste` / 键盘 IME 不可靠时，用快捷键直连剪贴板。

use std::process::Command;

/// Rich clipboard payload (macOS helper when available).
#[derive(Debug, Clone)]
pub(crate) enum ClipboardPayload {
    Text(String),
    Image { mime: String, bytes: Vec<u8> },
}

/// 读取主剪贴板为 UTF-8 文本（失败则 `None`）。
pub(crate) fn read_system_clipboard() -> Option<String> {
    read_clipboard_payload().and_then(|p| match p {
        ClipboardPayload::Text(t) if !t.trim().is_empty() => Some(t),
        _ => None,
    })
}

/// Read clipboard text or image via macOS helper / platform tools.
pub(crate) fn read_clipboard_payload() -> Option<ClipboardPayload> {
    #[cfg(target_os = "macos")]
    {
        if crate::apple_media::apple_media_ocr_available() {
            if let Ok(items) =
                anycode_apple_media::read_pasteboard(anycode_apple_media::NO_EXTRA_PATHS)
            {
                for item in items {
                    if item.kind == "text" {
                        if let Some(text) = item.text.filter(|t| !t.trim().is_empty()) {
                            return Some(ClipboardPayload::Text(text));
                        }
                    }
                    if item.kind == "image" {
                        if let (Some(mime), Some(b64)) = (item.mime_type, item.data_base64) {
                            if let Ok(bytes) = base64::Engine::decode(
                                &base64::engine::general_purpose::STANDARD,
                                b64.trim(),
                            ) {
                                return Some(ClipboardPayload::Image { mime, bytes });
                            }
                        }
                    }
                }
            }
        }
        let out = Command::new("pbpaste").output().ok()?;
        if !out.status.success() {
            return None;
        }
        return String::from_utf8(out.stdout)
            .ok()
            .filter(|t| !t.is_empty())
            .map(ClipboardPayload::Text);
    }
    #[cfg(target_os = "linux")]
    {
        for (cmd, args) in [
            ("wl-paste", vec!["--no-newline"]),
            ("xclip", vec!["-selection", "clipboard", "-o"]),
            ("xsel", vec!["--clipboard", "--output"]),
        ] {
            if let Ok(out) = Command::new(cmd).args(&args).output() {
                if out.status.success() {
                    return String::from_utf8(out.stdout)
                        .ok()
                        .filter(|t| !t.is_empty())
                        .map(ClipboardPayload::Text);
                }
            }
        }
        None
    }
    #[cfg(windows)]
    {
        let out = Command::new("powershell")
            .args(["-NoProfile", "-Command", "Get-Clipboard -Raw"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        return String::from_utf8(out.stdout)
            .ok()
            .filter(|t| !t.is_empty())
            .map(ClipboardPayload::Text);
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "linux"), not(windows)))]
    {
        None
    }
}
