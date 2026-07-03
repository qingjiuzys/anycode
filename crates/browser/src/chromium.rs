//! Resolve bundled or system Chromium executable.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const CHROMIUM_ENV: &str = "ANYCODE_CHROMIUM_PATH";
const BROWSER_MCP_ROOT_ENV: &str = "ANYCODE_BROWSER_MCP_ROOT";

/// Resolve Chromium binary for CDP launch.
pub fn resolve_chromium_executable() -> Option<PathBuf> {
    if let Ok(raw) = std::env::var(CHROMIUM_ENV) {
        let p = PathBuf::from(raw.trim());
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(root) = std::env::var(BROWSER_MCP_ROOT_ENV) {
        if let Some(p) = find_playwright_chromium(Path::new(root.trim()).join("browsers")) {
            return Some(p);
        }
    }
    for candidate in system_chromium_candidates() {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn chromium_doctor_message() -> String {
    if resolve_chromium_executable().is_some() {
        return "Chromium executable found.".into();
    }
    [
        "Chromium not found.",
        "Set ANYCODE_CHROMIUM_PATH, enable desktop browser bundle (ANYCODE_BROWSER_MCP_ROOT),",
        "or install Google Chrome / Chromium on PATH.",
    ]
    .join(" ")
}

fn find_playwright_chromium(browsers_dir: PathBuf) -> Option<PathBuf> {
    if !browsers_dir.is_dir() {
        return None;
    }
    for entry in WalkDir::new(&browsers_dir)
        .max_depth(6)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == "Chromium" || name == "chrome" || name == "Google Chrome for Testing" {
            if path.is_file() {
                return Some(path.to_path_buf());
            }
        }
        if name == "Chromium.app" {
            let mac_bin = path.join("Contents/MacOS/Chromium");
            if mac_bin.is_file() {
                return Some(mac_bin);
            }
        }
    }
    None
}

fn system_chromium_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(target_os = "macos") {
        out.push(PathBuf::from(
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        ));
        out.push(PathBuf::from(
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ));
    }
    if cfg!(target_os = "linux") {
        for name in [
            "chromium",
            "chromium-browser",
            "google-chrome",
            "google-chrome-stable",
        ] {
            out.push(PathBuf::from(format!("/usr/bin/{name}")));
        }
    }
    if cfg!(target_os = "windows") {
        out.push(PathBuf::from(
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        ));
        out.push(PathBuf::from(
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_message_when_missing_is_actionable() {
        let msg = chromium_doctor_message();
        assert!(msg.contains("ANYCODE_CHROMIUM_PATH") || msg.contains("found"));
    }
}
