//! Outbound deliverable hints and inline file bodies for WeChat replies.

use super::ilink::WxSender;
use anyhow::Result;
use std::path::PathBuf;

const INLINE_FILE_MAX_BYTES: u64 = 24_000;
const CDN_FILE_MAX_BYTES: u64 = 10 * 1024 * 1024;
const CHUNK_MAX: usize = 2048;

pub fn with_deliverable_hint(reply: String, raw_output: &str) -> String {
    if let Some(path) = extract_deliverable_path(raw_output) {
        format!("{reply}\n\n📎 {path}")
    } else {
        reply
    }
}

pub fn extract_deliverable_path(text: &str) -> Option<String> {
    for token in text.split_whitespace() {
        let t = token.trim_matches(|c: char| c == '"' || c == '\'' || c == '`' || c == ',');
        if (t.ends_with(".md") || t.ends_with(".pdf") || t.ends_with(".txt"))
            && (t.starts_with('/') || t.starts_with('~') || t.contains('/'))
        {
            return Some(t.to_string());
        }
    }
    None
}

pub fn resolve_deliverable_path(token: &str) -> Option<PathBuf> {
    let t = token.trim();
    if t.is_empty() {
        return None;
    }
    let path = if let Some(rest) = t.strip_prefix('~') {
        dirs::home_dir()?.join(rest.trim_start_matches('/'))
    } else {
        PathBuf::from(t)
    };
    path.canonicalize().ok().filter(|p| p.is_file())
}

pub async fn send_deliverable_file(
    sender: &WxSender,
    to_user_id: &str,
    context_token: &str,
    path_token: &str,
) -> Result<()> {
    let Some(path) = resolve_deliverable_path(path_token) else {
        return Ok(());
    };
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let meta = std::fs::metadata(&path)?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if meta.len() <= INLINE_FILE_MAX_BYTES && matches!(ext.as_str(), "md" | "txt" | "markdown") {
        let content = std::fs::read_to_string(&path)?;
        let msg = format!("📎 {name}\n\n{content}");
        for chunk in split_message(&msg, CHUNK_MAX) {
            sender.send_text(to_user_id, context_token, &chunk).await?;
        }
        return Ok(());
    }
    if meta.len() <= CDN_FILE_MAX_BYTES {
        match std::fs::read(&path) {
            Ok(bytes) => {
                if let Err(e) = sender
                    .send_file(to_user_id, context_token, name, &bytes)
                    .await
                {
                    tracing::warn!(error = %e, path = %path.display(), "CDN file send failed, falling back to path note");
                } else {
                    sender
                        .send_text(to_user_id, context_token, &format!("📎 已发送文件：{name}"))
                        .await?;
                    return Ok(());
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "read deliverable failed");
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

    #[test]
    fn extract_path_from_output() {
        let p = extract_deliverable_path("saved to /tmp/out/report.md ok").unwrap();
        assert_eq!(p, "/tmp/out/report.md");
    }
}
