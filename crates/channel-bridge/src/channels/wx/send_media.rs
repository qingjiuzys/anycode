//! Outbound media routing (image / video / file), aligned with openclaw-weixin `send-media.ts`.

use super::cdn_upload::{
    upload_bytes_to_cdn_with_media_type, UPLOAD_MEDIA_FILE, UPLOAD_MEDIA_IMAGE, UPLOAD_MEDIA_VIDEO,
};
use super::ilink::WxSender;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboundMediaKind {
    Image,
    Video,
    File,
}

/// Classify outbound media by file extension (openclaw-weixin `getMimeFromFilename` subset).
pub fn outbound_media_kind_for_path(path: &Path) -> OutboundMediaKind {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "mp4" | "mov" | "m4v" | "webm" | "avi" | "mkv" => OutboundMediaKind::Video,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "heic" | "heif" => {
            OutboundMediaKind::Image
        }
        _ => OutboundMediaKind::File,
    }
}

pub fn upload_media_type_for_kind(kind: OutboundMediaKind) -> i64 {
    match kind {
        OutboundMediaKind::Image => UPLOAD_MEDIA_IMAGE,
        OutboundMediaKind::Video => UPLOAD_MEDIA_VIDEO,
        OutboundMediaKind::File => UPLOAD_MEDIA_FILE,
    }
}

/// Upload a local file and send as WeChat media (image / video / file item).
pub async fn send_weixin_media_file(
    sender: &WxSender,
    to_user_id: &str,
    context_token: &str,
    file_path: &Path,
    caption: &str,
) -> Result<()> {
    let bytes = std::fs::read(file_path)
        .with_context(|| format!("read media file {}", file_path.display()))?;
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let kind = outbound_media_kind_for_path(file_path);
    let media_type = upload_media_type_for_kind(kind);
    let uploaded =
        upload_bytes_to_cdn_with_media_type(sender.api(), &bytes, to_user_id, media_type).await?;

    if !caption.trim().is_empty() {
        sender.send_text(to_user_id, context_token, caption).await?;
    }

    match kind {
        OutboundMediaKind::Image => {
            sender
                .send_image_message(to_user_id, context_token, &uploaded)
                .await?;
        }
        OutboundMediaKind::Video => {
            sender
                .send_video_message(to_user_id, context_token, &uploaded)
                .await?;
        }
        OutboundMediaKind::File => {
            sender
                .send_file_message(to_user_id, context_token, file_name, &uploaded)
                .await?;
        }
    }
    Ok(())
}

/// Resolve a path token against optional working directory.
pub fn resolve_media_path(token: &str, cwd: Option<&Path>) -> Option<PathBuf> {
    let t = token.trim();
    if t.is_empty() {
        return None;
    }
    let path = if let Some(rest) = t.strip_prefix('~') {
        dirs::home_dir()?.join(rest.trim_start_matches('/'))
    } else if Path::new(t).is_absolute() {
        PathBuf::from(t)
    } else if let Some(base) = cwd {
        base.join(t)
    } else {
        PathBuf::from(t)
    };
    path.canonicalize().ok().filter(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_extensions_route_to_image() {
        assert_eq!(
            outbound_media_kind_for_path(Path::new("/tmp/a.PNG")),
            OutboundMediaKind::Image
        );
    }

    #[test]
    fn video_extensions_route_to_video() {
        assert_eq!(
            outbound_media_kind_for_path(Path::new("clip.mp4")),
            OutboundMediaKind::Video
        );
    }

    #[test]
    fn pdf_routes_to_file() {
        assert_eq!(
            outbound_media_kind_for_path(Path::new("/tmp/report.pdf")),
            OutboundMediaKind::File
        );
        assert_eq!(upload_media_type_for_kind(OutboundMediaKind::File), 3);
    }
}
