//! Outbound deliverable hints and media file delivery for WeChat replies.

use super::ilink::WxSender;
use super::send_media::{resolve_media_path, send_weixin_media_file};
use anycode_core::Artifact;
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const INLINE_FILE_MAX_BYTES: u64 = 24_000;
pub const CDN_FILE_MAX_BYTES: u64 = 10 * 1024 * 1024;
const CHUNK_MAX: usize = 2048;

const DELIVERABLE_EXTS: &[&str] = &[
    "md", "txt", "markdown", "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "zip", "tar",
    "gz", "7z", "json", "csv", "png", "jpg", "jpeg", "gif", "webp", "bmp", "mp4", "mov", "m4v",
    "webm",
];

fn is_deliverable_extension(ext: &str) -> bool {
    DELIVERABLE_EXTS.contains(&ext.to_ascii_lowercase().as_str())
}

fn trim_path_token(token: &str) -> &str {
    token.trim_matches(|c: char| c == '"' || c == '\'' || c == '`' || c == ',' || c == ';')
}

fn looks_like_path_token(token: &str) -> bool {
    token.starts_with('/')
        || token.starts_with('~')
        || token.contains('/')
        || token.contains('\\')
        || token.starts_with("./")
        || token.starts_with("../")
}

pub fn extract_deliverable_paths(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for token in text.split_whitespace() {
        let t = trim_path_token(token);
        if t.is_empty() {
            continue;
        }
        let ext = Path::new(t)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if is_deliverable_extension(ext) && looks_like_path_token(t) {
            out.push(t.to_string());
        }
    }
    out
}

/// First deliverable path in text (backward compatible).
pub fn extract_deliverable_path(text: &str) -> Option<String> {
    extract_deliverable_paths(text).into_iter().next()
}

pub fn resolve_deliverable_path(token: &str) -> Option<PathBuf> {
    resolve_media_path(token, None)
}

/// Collect unique canonical file paths from agent artifacts and output text.
pub fn collect_outbound_media_paths(
    artifacts: &[Artifact],
    output: &str,
    cwd: Option<&Path>,
) -> Vec<PathBuf> {
    let mut seen = HashSet::<PathBuf>::new();
    let mut paths = Vec::new();

    let mut push = |candidate: PathBuf| {
        if seen.insert(candidate.clone()) {
            paths.push(candidate);
        }
    };

    for artifact in artifacts {
        if let Some(ref token) = artifact.path {
            if let Some(p) = resolve_media_path(token, cwd) {
                push(p);
            }
        }
    }

    for token in extract_deliverable_paths(output) {
        if let Some(p) = resolve_media_path(&token, cwd) {
            push(p);
        }
    }

    paths
}

pub fn with_deliverable_hint(reply: String, raw_output: &str) -> String {
    let paths = extract_deliverable_paths(raw_output);
    if paths.is_empty() {
        return reply;
    }
    let hints: Vec<String> = paths.into_iter().map(|p| format!("📎 {p}")).collect();
    format!("{reply}\n\n{}", hints.join("\n"))
}

pub async fn send_deliverable_file(
    sender: &WxSender,
    to_user_id: &str,
    context_token: &str,
    path_token: &str,
) -> Result<()> {
    let Some(path) = resolve_media_path(path_token, None) else {
        return Ok(());
    };
    send_deliverable_path(sender, to_user_id, context_token, &path).await
}

pub async fn send_outbound_media_paths(
    sender: &WxSender,
    to_user_id: &str,
    context_token: &str,
    paths: &[PathBuf],
) -> Result<()> {
    for path in paths {
        if let Err(e) = send_deliverable_path(sender, to_user_id, context_token, path).await {
            tracing::warn!(
                error = %e,
                path = %path.display(),
                "wx outbound media send failed"
            );
        }
    }
    Ok(())
}

async fn send_deliverable_path(
    sender: &WxSender,
    to_user_id: &str,
    context_token: &str,
    path: &Path,
) -> Result<()> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let meta = std::fs::metadata(path)?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if meta.len() <= INLINE_FILE_MAX_BYTES && matches!(ext.as_str(), "md" | "txt" | "markdown") {
        let content = std::fs::read_to_string(path)?;
        let msg = format!("📎 {name}\n\n{content}");
        for chunk in split_message(&msg, CHUNK_MAX) {
            sender.send_text(to_user_id, context_token, &chunk).await?;
        }
        return Ok(());
    }

    if meta.len() <= CDN_FILE_MAX_BYTES {
        match send_weixin_media_file(sender, to_user_id, context_token, path, "").await {
            Ok(()) => {
                sender
                    .send_text(to_user_id, context_token, &format!("📎 已发送：{name}"))
                    .await?;
                return Ok(());
            }
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "CDN media send failed");
            }
        }
    }

    let size_kb = meta.len() / 1024;
    let note = format!(
        "📎 {name} ({size_kb} KB)\n路径：{}\n（文件过大或 CDN 上传失败时请在本机打开）",
        path.display()
    );
    sender.send_text(to_user_id, context_token, &note).await?;
    Ok(())
}

fn split_message(text: &str, max_chars: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for ch in text.chars() {
        buf.push(ch);
        if buf.chars().count() >= max_chars {
            out.push(buf.trim_end().to_string());
            buf.clear();
        }
    }
    if !buf.trim().is_empty() {
        out.push(buf.trim_end().to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_core::Artifact;
    use std::collections::HashMap;

    #[test]
    fn extract_path_from_output() {
        let p = extract_deliverable_path("saved to /tmp/out/report.md ok").unwrap();
        assert_eq!(p, "/tmp/out/report.md");
    }

    #[test]
    fn extract_multiple_extensions() {
        let paths = extract_deliverable_paths("see /tmp/a.png and ~/b.pdf done");
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|p| p.ends_with("a.png")));
        assert!(paths.iter().any(|p| p.ends_with("b.pdf")));
    }

    #[test]
    fn collect_paths_from_artifacts_and_text() {
        let dir =
            std::env::temp_dir().join(format!("anycode-wx-deliverable-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("out.json");
        std::fs::write(&file, "{}").unwrap();
        let artifact = Artifact {
            name: "file".into(),
            path: Some(file.display().to_string()),
            content: None,
            metadata: HashMap::new(),
        };
        let got = collect_outbound_media_paths(&[artifact], "", Some(dir.as_path()));
        assert_eq!(got.len(), 1);
        assert_eq!(got[0], file.canonicalize().unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
