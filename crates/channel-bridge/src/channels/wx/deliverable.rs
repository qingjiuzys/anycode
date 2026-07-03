//! Outbound deliverable hints and media file delivery for WeChat replies.

use super::ilink::WxSender;
use super::send_media::{resolve_media_path, send_weixin_media_file};
use super::store::SessionDeliverable;
use anycode_core::Artifact;
use anycode_tools::WeChatMediaDelivery;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

const REMOTE_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);

const INLINE_FILE_MAX_BYTES: u64 = 24_000;
pub const CDN_FILE_MAX_BYTES: u64 = 10 * 1024 * 1024;
const CHUNK_MAX: usize = 2048;

const EXCEL_EXTS: &[&str] = &["xls", "xlsx", "xlsm", "csv"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResendFileKind {
    Any,
    Excel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResendIntent {
    pub kind: ResendFileKind,
}

const RESEND_KEYWORDS: &[&str] = &[
    "继续发",
    "再发",
    "重发",
    "发一下",
    "发上来",
    "刚才那个",
    "上一份",
    "上一个",
    "刚才的",
    "resend",
    "send again",
    "send it again",
];

const EXCEL_KEYWORDS: &[&str] = &["excel", "xlsx", "xls", "表格", "电子表格", "spreadsheet"];

pub fn detect_resend_request(text: &str) -> Option<ResendIntent> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();
    if !RESEND_KEYWORDS.iter().any(|k| lower.contains(k)) {
        return None;
    }
    let kind = if EXCEL_KEYWORDS.iter().any(|k| lower.contains(k)) {
        ResendFileKind::Excel
    } else {
        ResendFileKind::Any
    };
    Some(ResendIntent { kind })
}

pub fn match_deliverable_for_resend(
    deliverables: &[SessionDeliverable],
    intent: ResendIntent,
) -> Option<PathBuf> {
    for item in deliverables.iter().rev() {
        if !extension_matches_resend(&item.extension, intent.kind) {
            continue;
        }
        let path = PathBuf::from(&item.path);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn extension_matches_resend(ext: &str, kind: ResendFileKind) -> bool {
    match kind {
        ResendFileKind::Any => is_deliverable_extension(ext),
        ResendFileKind::Excel => EXCEL_EXTS.contains(&ext.to_ascii_lowercase().as_str()),
    }
}

pub fn assistant_history_with_paths(reply: &str, paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        return reply.to_string();
    }
    let hints: Vec<String> = paths
        .iter()
        .map(|p| format!("📎 {}", p.display()))
        .collect();
    if reply.trim().is_empty() {
        hints.join("\n")
    } else {
        format!("{reply}\n\n{}", hints.join("\n"))
    }
}

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
        if is_deliverable_remote_url(t) {
            out.push(t.to_string());
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

/// http(s) URLs with a deliverable media extension in the path.
pub fn is_deliverable_remote_url(token: &str) -> bool {
    let t = trim_path_token(token);
    if !(t.starts_with("http://") || t.starts_with("https://")) {
        return false;
    }
    let path = t.split('?').next().unwrap_or(t);
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    is_deliverable_extension(ext)
}

pub fn extract_deliverable_urls(text: &str) -> Vec<String> {
    extract_deliverable_paths(text)
        .into_iter()
        .filter(|t| is_deliverable_remote_url(t))
        .collect()
}

fn extension_from_content_type_or_url(content_type: Option<&str>, url: &str) -> String {
    if let Some(ct) = content_type {
        let ct = ct
            .split(';')
            .next()
            .unwrap_or(ct)
            .trim()
            .to_ascii_lowercase();
        let ext = match ct.as_str() {
            "video/mp4" => "mp4",
            "video/quicktime" => "mov",
            "video/webm" => "webm",
            "image/png" => "png",
            "image/jpeg" | "image/jpg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "application/pdf" => "pdf",
            _ => "",
        };
        if !ext.is_empty() {
            return ext.to_string();
        }
    }
    url.split('?')
        .next()
        .and_then(|base| Path::new(base).extension())
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .filter(|e| is_deliverable_extension(e))
        .unwrap_or_else(|| "bin".to_string())
}

fn weixin_outbound_temp_dir() -> Result<PathBuf> {
    let dir = dirs::home_dir()
        .context("home dir")?
        .join(".anycode/tmp/weixin-outbound");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

async fn download_remote_media_to_temp(url: &str) -> Result<PathBuf> {
    let trimmed = url.trim();
    if !is_deliverable_remote_url(trimmed) {
        anyhow::bail!("unsupported remote media URL");
    }
    let client = reqwest::Client::builder()
        .user_agent(anycode_core::user_agent("anycode-wx"))
        .timeout(REMOTE_DOWNLOAD_TIMEOUT)
        .build()?;
    let resp = client
        .get(trimmed)
        .send()
        .await
        .with_context(|| format!("GET {trimmed}"))?;
    if !resp.status().is_success() {
        anyhow::bail!("remote media download HTTP {}", resp.status());
    }
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let bytes = resp.bytes().await?;
    if bytes.len() as u64 > CDN_FILE_MAX_BYTES {
        anyhow::bail!(
            "remote media too large ({} bytes, max {CDN_FILE_MAX_BYTES})",
            bytes.len()
        );
    }
    let ext = extension_from_content_type_or_url(content_type.as_deref(), trimmed);
    let dir = weixin_outbound_temp_dir()?;
    let name = format!(
        "weixin-remote-{}-{}.{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis(),
        ext
    );
    let path = dir.join(name);
    std::fs::write(&path, &bytes)?;
    Ok(path)
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

/// Local paths plus downloaded remote http(s) media URLs.
pub async fn resolve_outbound_media_paths(
    artifacts: &[Artifact],
    output: &str,
    cwd: Option<&Path>,
) -> Vec<PathBuf> {
    let mut paths = collect_outbound_media_paths(artifacts, output, cwd);
    let mut seen = paths.iter().cloned().collect::<HashSet<_>>();
    for url in extract_deliverable_urls(output) {
        match download_remote_media_to_temp(&url).await {
            Ok(path) => {
                if seen.insert(path.clone()) {
                    paths.push(path);
                }
            }
            Err(e) => {
                tracing::warn!(url = %url, error = %e, "wx remote media download failed");
            }
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
    send_deliverable_path(sender, to_user_id, context_token, &path, None).await?;
    Ok(())
}

pub async fn send_outbound_media_paths(
    sender: &WxSender,
    to_user_id: &str,
    context_token: &str,
    paths: &[PathBuf],
) -> Result<()> {
    for path in paths {
        if let Err(e) = send_deliverable_path(sender, to_user_id, context_token, path, None).await {
            tracing::warn!(
                error = %e,
                path = %path.display(),
                "wx outbound media send failed"
            );
        }
    }
    Ok(())
}

pub async fn send_deliverable_path(
    sender: &WxSender,
    to_user_id: &str,
    context_token: &str,
    path: &Path,
    caption: Option<&str>,
) -> Result<WeChatMediaDelivery> {
    if let Some(cap) = caption.map(str::trim).filter(|s| !s.is_empty()) {
        sender.send_text(to_user_id, context_token, cap).await?;
    }

    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let meta = std::fs::metadata(path)?;
    let size_kb = meta.len() / 1024;
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
        return Ok(WeChatMediaDelivery::InlineText);
    }

    if meta.len() <= CDN_FILE_MAX_BYTES {
        match send_weixin_media_file(sender, to_user_id, context_token, path, "").await {
            Ok(()) => {
                sender
                    .send_text(to_user_id, context_token, &format!("📎 已发送：{name}"))
                    .await?;
                return Ok(WeChatMediaDelivery::CdnMedia);
            }
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "CDN media send failed");
                let reason = compact_cdn_error(&e);
                let note = format!(
                    "📎 {name} ({size_kb} KB)\nCDN 上传失败：{reason}\n路径：{}\n（请在本机打开）",
                    path.display()
                );
                sender.send_text(to_user_id, context_token, &note).await?;
                return Ok(WeChatMediaDelivery::PathNote);
            }
        }
    }

    let note = format!(
        "📎 {name} ({size_kb} KB)\n文件超过微信 CDN 上限（{CDN_MB} MB）\n路径：{}\n（请在本机打开）",
        path.display()
    );
    sender.send_text(to_user_id, context_token, &note).await?;
    Ok(WeChatMediaDelivery::PathNote)
}

fn compact_cdn_error(err: &anyhow::Error) -> String {
    let s = err.to_string();
    s.chars().take(200).collect()
}

const CDN_MB: u64 = CDN_FILE_MAX_BYTES / (1024 * 1024);

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
    fn extract_remote_video_url() {
        let url = "https://cdn.example.com/videos/clip.mp4?sig=abc";
        assert!(is_deliverable_remote_url(url));
        let paths = extract_deliverable_urls(&format!("ready at {url}"));
        assert_eq!(paths, vec![url]);
    }

    #[test]
    fn extension_from_url_path() {
        assert_eq!(
            extension_from_content_type_or_url(
                None,
                "https://cdn.example.com/a/b/clip.mp4?token=1"
            ),
            "mp4"
        );
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

    #[test]
    fn detect_resend_excel_request() {
        let intent = detect_resend_request("继续发一下，excel").expect("intent");
        assert_eq!(intent.kind, ResendFileKind::Excel);
    }

    #[test]
    fn match_latest_excel_deliverable() {
        use super::super::store::{DeliverableSource, SessionDeliverable};
        let deliverables = vec![
            SessionDeliverable {
                path: "/tmp/old.xlsx".into(),
                file_name: "old.xlsx".into(),
                extension: "xlsx".into(),
                mime_type: None,
                source: DeliverableSource::Outbound,
                sent: true,
                description: None,
                timestamp: 1,
            },
            SessionDeliverable {
                path: "/tmp/new.xlsx".into(),
                file_name: "new.xlsx".into(),
                extension: "xlsx".into(),
                mime_type: None,
                source: DeliverableSource::Outbound,
                sent: true,
                description: None,
                timestamp: 2,
            },
        ];
        let dir = std::env::temp_dir().join(format!("anycode-wx-resend-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("new.xlsx");
        std::fs::write(&file, b"xlsx").unwrap();
        let mut items = deliverables;
        items[1].path = file.display().to_string();
        let got = match_deliverable_for_resend(
            &items,
            ResendIntent {
                kind: ResendFileKind::Excel,
            },
        )
        .unwrap();
        assert_eq!(got, file);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
