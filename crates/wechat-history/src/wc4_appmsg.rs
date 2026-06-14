//! Parse WeChat app/file message XML payloads (local_type 49 and related).

use crate::model::AttachmentKind;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct AppMsgMeta {
    pub title: Option<String>,
    pub app_type: Option<i64>,
    pub file_ext: Option<String>,
    pub total_len: Option<u64>,
    pub attach_id: Option<String>,
    pub md5: Option<String>,
    pub url: Option<String>,
    pub kind: AttachmentKind,
}

/// Extract plain text or XML payload from message row fields.
pub fn extract_payload(message_content: &str, compress_content: Option<&str>) -> String {
    let mc = message_content.trim();
    if !mc.is_empty() && (mc.starts_with('<') || mc.contains("<appmsg")) {
        return mc.to_string();
    }
    if let Some(cc) = compress_content.filter(|s| !s.trim().is_empty()) {
        if cc.starts_with('<') || cc.contains("<appmsg") {
            return cc.to_string();
        }
    }
    mc.to_string()
}

pub fn parse_appmsg_xml(payload: &str) -> Option<AppMsgMeta> {
    let payload = payload.trim();
    if payload.is_empty() {
        return None;
    }
    let mut meta = AppMsgMeta::default();
    meta.title = tag_text(payload, "title");
    meta.app_type = tag_text(payload, "type").and_then(|s| s.parse().ok());
    meta.file_ext = tag_text(payload, "fileext");
    meta.total_len = tag_text(payload, "totallen").and_then(|s| s.parse().ok());
    meta.attach_id = tag_text(payload, "attachid");
    meta.md5 = tag_text(payload, "md5");
    meta.url = tag_text(payload, "url");
    meta.kind = classify_app_type(
        meta.app_type,
        meta.file_ext.as_deref(),
        meta.title.as_deref(),
    );
    if meta.title.is_some() || meta.file_ext.is_some() || meta.attach_id.is_some() {
        Some(meta)
    } else {
        None
    }
}

fn classify_app_type(
    app_type: Option<i64>,
    file_ext: Option<&str>,
    title: Option<&str>,
) -> AttachmentKind {
    match app_type {
        Some(6) => AttachmentKind::File,
        Some(5) => AttachmentKind::Link,
        Some(33) | Some(36) => AttachmentKind::MiniProgram,
        Some(3) => AttachmentKind::Image,
        Some(34) => AttachmentKind::Voice,
        Some(43) => AttachmentKind::Video,
        _ => {
            let ext = file_ext.unwrap_or("").to_ascii_lowercase();
            let name = title.unwrap_or("").to_ascii_lowercase();
            if is_spreadsheet_ext(&ext) || name.ends_with(".xlsx") || name.ends_with(".xls") {
                AttachmentKind::File
            } else if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp") {
                AttachmentKind::Image
            } else if matches!(ext.as_str(), "mp4" | "mov") {
                AttachmentKind::Video
            } else if !ext.is_empty() || name.contains('.') {
                AttachmentKind::File
            } else {
                AttachmentKind::Unknown
            }
        }
    }
}

pub fn is_spreadsheet_ext(ext: &str) -> bool {
    matches!(ext.to_ascii_lowercase().as_str(), "xlsx" | "xls" | "csv")
}

pub fn infer_file_name(meta: &AppMsgMeta) -> Option<String> {
    meta.title.clone().or_else(|| {
        meta.file_ext
            .as_ref()
            .map(|ext| format!("attachment.{ext}"))
    })
}

pub fn infer_extension(file_name: &str, meta: &AppMsgMeta) -> Option<String> {
    Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_string)
        .or_else(|| meta.file_ext.clone())
}

fn tag_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    let value = xml[start..end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_file_appmsg() {
        let xml = r#"<msg><appmsg><title>订单明细.xlsx</title><type>6</type><appattach><totallen>12345</totallen><fileext>xlsx</fileext><attachid>abc</attachid></appattach></appmsg></msg>"#;
        let meta = parse_appmsg_xml(xml).unwrap();
        assert_eq!(meta.title.as_deref(), Some("订单明细.xlsx"));
        assert_eq!(meta.file_ext.as_deref(), Some("xlsx"));
        assert_eq!(meta.kind, AttachmentKind::File);
    }

    #[test]
    fn classify_spreadsheet_by_name() {
        let meta = parse_appmsg_xml(
            r#"<appmsg><title>sales.xls</title><type>6</type><fileext>xls</fileext></appmsg>"#,
        )
        .unwrap();
        assert_eq!(meta.kind, AttachmentKind::File);
    }
}
