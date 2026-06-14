//! Resolve WeChat attachment local paths and enrich messages with file metadata.

use crate::file_parser::{parse_local_file, ParseLimits};
use crate::model::{
    AttachmentKind, AttachmentParseStatus, WechatAttachment, WechatChatMessage, WechatHistoryQuery,
};
use crate::name_resolver::NameResolver;
use crate::wc4_appmsg::{infer_extension, infer_file_name, parse_appmsg_xml, AppMsgMeta};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct AttachmentContext {
    pub data_dir: PathBuf,
    pub wechat_root: Option<PathBuf>,
    pub names: NameResolver,
    pub limits: ParseLimits,
}

impl AttachmentContext {
    pub fn new(data_dir: PathBuf, key_map: &HashMap<String, String>, max_bytes: u64) -> Self {
        Self {
            data_dir: data_dir.clone(),
            wechat_root: data_dir.parent().map(Path::to_path_buf),
            names: NameResolver::from_data_dir(&data_dir, key_map),
            limits: ParseLimits { max_bytes },
        }
    }
}

pub fn enrich_messages(
    ctx: &AttachmentContext,
    query: &WechatHistoryQuery,
    messages: &mut [WechatChatMessage],
) -> crate::model::AttachmentStats {
    let mut stats = crate::model::AttachmentStats {
        attachment_messages: 0,
        resolved_files: 0,
        parsed_files: 0,
        parse_failures: 0,
    };

    if !query.include_attachments {
        return stats;
    }

    let type_filter = query
        .attachment_types
        .as_ref()
        .map(|v| v.iter().map(|s| s.to_ascii_lowercase()).collect::<Vec<_>>());

    for msg in messages.iter_mut() {
        if msg.attachments.is_empty() {
            continue;
        }
        if let Some(ref allowed) = type_filter {
            msg.attachments
                .retain(|a| allowed.iter().any(|t| t == "all" || t == a.kind.as_str()));
            if msg.attachments.is_empty() {
                continue;
            }
        }
        stats.attachment_messages += 1;
        for att in msg.attachments.iter_mut() {
            att.conversation_id = msg.conversation_id.clone();
            att.conversation_name = ctx
                .names
                .conversation_name(&msg.conversation_id)
                .or_else(|| msg.conversation_name.clone());
            if let Some(ref sid) = att.sender_id {
                att.sender_name = ctx
                    .names
                    .sender_name(sid)
                    .or_else(|| att.sender_name.clone());
            }
            if att.sender_name.is_none() {
                att.sender_name = msg.sender.clone();
            }

            if let Some(path) = resolve_local_path(ctx, att) {
                att.local_path = Some(path.display().to_string());
                stats.resolved_files += 1;
                att.sha256 = sha256_file(&path).ok();

                if query.parse_files && att.kind == AttachmentKind::File {
                    let (status, parsed, err) =
                        parse_local_file(&path, att.extension.as_deref(), &ctx.limits);
                    att.parse_status = status;
                    att.parsed = parsed;
                    att.error = err;
                    match status {
                        AttachmentParseStatus::Parsed => stats.parsed_files += 1,
                        AttachmentParseStatus::ParseFailed | AttachmentParseStatus::Unsupported => {
                            stats.parse_failures += 1;
                        }
                        _ => {}
                    }
                } else if att.kind == AttachmentKind::File {
                    att.parse_status = AttachmentParseStatus::MetadataOnly;
                }
            } else if att.kind == AttachmentKind::File {
                att.parse_status = AttachmentParseStatus::MissingFile;
            }

            if msg.media_path.is_none() {
                msg.media_path = att.local_path.clone();
            }
        }
    }
    stats
}

pub fn attachment_from_appmsg(
    meta: &AppMsgMeta,
    conversation_id: &str,
    sender_id: Option<String>,
) -> WechatAttachment {
    let file_name = infer_file_name(meta);
    let extension = file_name.as_deref().and_then(|n| infer_extension(n, meta));
    let mime = extension.as_ref().map(|ext| mime_guess(ext));
    WechatAttachment {
        kind: meta.kind,
        file_name,
        extension,
        mime,
        size_bytes: meta.total_len,
        sender_id,
        sender_name: None,
        conversation_id: conversation_id.to_string(),
        conversation_name: None,
        local_path: None,
        sha256: None,
        parse_status: AttachmentParseStatus::NotApplicable,
        parsed: None,
        attach_id: meta.attach_id.clone(),
        error: None,
    }
}

fn mime_guess(ext: &str) -> String {
    match ext.to_ascii_lowercase().as_str() {
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "xls" => "application/vnd.ms-excel",
        "csv" => "text/csv",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        other => other,
    }
    .to_string()
}

fn resolve_local_path(ctx: &AttachmentContext, att: &WechatAttachment) -> Option<PathBuf> {
    let file_name = att.file_name.as_deref()?;
    let mut candidates = Vec::new();
    if let Some(root) = &ctx.wechat_root {
        for sub in ["msg/file", "msg/attach", "FileStorage/File"] {
            candidates.push(root.join(sub).join(file_name));
        }
        if let Some(md5) = att.attach_id.as_deref().filter(|s| s.len() >= 8) {
            candidates.push(root.join("msg/file").join(md5));
        }
    }
    candidates.push(ctx.data_dir.join(file_name));
    for path in candidates {
        if path.is_file() {
            return Some(path);
        }
    }
    search_by_filename(ctx.wechat_root.as_deref(), file_name)
}

fn search_by_filename(root: Option<&Path>, file_name: &str) -> Option<PathBuf> {
    let root = root?;
    let mut stack = vec![root.to_path_buf()];
    let mut seen = 0usize;
    while let Some(dir) = stack.pop() {
        let Ok(read) = fs::read_dir(&dir) else {
            continue;
        };
        for ent in read.flatten() {
            let p = ent.path();
            if p.is_dir() {
                if seen < 5000 {
                    stack.push(p);
                    seen += 1;
                }
            } else if p.file_name().and_then(|n| n.to_str()) == Some(file_name) {
                return Some(p);
            }
        }
    }
    None
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let digest = Sha256::digest(bytes);
    Ok(format!("{:x}", digest))
}

pub fn build_attachment_from_payload(
    payload: &str,
    local_type: i64,
    conversation_id: &str,
    sender_id: Option<String>,
) -> Option<WechatAttachment> {
    if local_type == 49 || payload.contains("<appmsg") {
        let meta = parse_appmsg_xml(payload)?;
        return Some(attachment_from_appmsg(&meta, conversation_id, sender_id));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wc4_appmsg::parse_appmsg_xml;

    #[test]
    fn builds_attachment_from_xml() {
        let xml =
            r#"<appmsg><title>报价.xlsx</title><type>6</type><fileext>xlsx</fileext></appmsg>"#;
        let meta = parse_appmsg_xml(xml).unwrap();
        let att = attachment_from_appmsg(&meta, "wxid_a@chatroom", Some("wxid_b".into()));
        assert_eq!(att.file_name.as_deref(), Some("报价.xlsx"));
        assert_eq!(att.kind, AttachmentKind::File);
    }
}
