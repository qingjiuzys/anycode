//! 系统剪贴板读取：在 raw TTY 下 `Event::Paste` / 键盘 IME 不可靠时，用快捷键直连剪贴板。

use std::process::Command;

/// 读取主剪贴板为 UTF-8 文本（失败则 `None`）。
pub(crate) fn read_system_clipboard() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let out = Command::new("pbpaste").output().ok()?;
        if !out.status.success() {
            return None;
        }
        String::from_utf8(out.stdout).ok()
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
                    return String::from_utf8(out.stdout).ok();
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
        String::from_utf8(out.stdout).ok()
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "linux"), not(windows)))]
    {
        None
    }
}
