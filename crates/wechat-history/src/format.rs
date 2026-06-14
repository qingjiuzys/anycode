use crate::date_filter::{format_timestamp_ms, summarize_content};
use crate::model::{
    AttachmentParseStatus, ParsedFilePreview, WechatChatMessage, WechatHistoryQuery,
};
use chrono_tz::Tz;

pub fn render_markdown_table(
    messages: &[WechatChatMessage],
    query: &WechatHistoryQuery,
    tz: Tz,
) -> String {
    let mut lines = vec![
        format!("# WeChat daily history — {}", query.date.trim()),
        String::new(),
        "| Time | Conversation | Sender | Direction | Type | Summary |".to_string(),
        "| --- | --- | --- | --- | --- | --- |".to_string(),
    ];
    if messages.is_empty() {
        lines.push("| — | — | — | — | — | (no messages) |".to_string());
    } else {
        for m in messages {
            let conv = m
                .conversation_name
                .as_deref()
                .unwrap_or(m.conversation_id.as_str());
            let sender = if query.include_group_sender {
                m.sender.as_deref().unwrap_or("—")
            } else {
                m.sender.as_deref().unwrap_or("—")
            };
            let summary =
                attachment_summary(m).unwrap_or_else(|| summarize_content(&m.content, 120));
            lines.push(format!(
                "| {} | {} | {} | {} | {} | {} |",
                format_timestamp_ms(m.timestamp_ms, tz),
                escape_cell(conv),
                escape_cell(sender),
                m.direction.as_label(),
                escape_cell(&m.msg_type),
                escape_cell(&summary),
            ));
        }
    }
    lines.join("\n")
}

fn attachment_summary(msg: &WechatChatMessage) -> Option<String> {
    let att = msg.attachments.first()?;
    let file_name = att.file_name.as_deref().unwrap_or("(file)");
    let status = match att.parse_status {
        AttachmentParseStatus::Parsed => "parsed",
        AttachmentParseStatus::MetadataOnly => "metadata",
        AttachmentParseStatus::MissingFile => "missing",
        AttachmentParseStatus::Unsupported => "unsupported",
        AttachmentParseStatus::ParseFailed => "parse_failed",
        AttachmentParseStatus::NotApplicable => "n/a",
    };
    let preview = att
        .parsed
        .as_ref()
        .map(preview_one_line)
        .unwrap_or_default();
    Some(format!("[file:{file_name} status:{status}] {preview}"))
}

fn preview_one_line(parsed: &ParsedFilePreview) -> String {
    match parsed {
        ParsedFilePreview::Excel(excel) => {
            let headers = excel.header_row.join(",");
            format!(
                "sheets={} rows~{} cols~{} headers=[{}]",
                excel.sheet_names.len(),
                excel.row_count,
                excel.column_count,
                summarize_content(&headers, 60)
            )
        }
        ParsedFilePreview::Text(text) => {
            format!(
                "lines={} {}",
                text.line_count,
                text.preview_lines.first().cloned().unwrap_or_default()
            )
        }
        ParsedFilePreview::Json(v) => summarize_content(&v.to_string(), 80),
    }
}

fn escape_cell(raw: &str) -> String {
    raw.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AttachmentKind, AttachmentParseStatus, MessageDirection, WechatAttachment,
        WechatHistoryQuery,
    };

    #[test]
    fn table_has_header_and_row() {
        let query = WechatHistoryQuery {
            date: "2026-06-14".into(),
            conversation: None,
            keyword: None,
            timezone: Some("Asia/Shanghai".into()),
            limit: None,
            include_group_sender: true,
            ..Default::default()
        };
        let tz: Tz = "Asia/Shanghai".parse().unwrap();
        let md = render_markdown_table(
            &[WechatChatMessage {
                id: None,
                timestamp_ms: 1_718_800_000_000,
                conversation_id: "wxid_a".into(),
                conversation_name: Some("Alice".into()),
                sender: Some("Alice".into()),
                sender_id: None,
                direction: MessageDirection::Inbound,
                msg_type: "text".into(),
                content: "hello".into(),
                media_path: None,
                attachments: Vec::new(),
            }],
            &query,
            tz,
        );
        assert!(md.contains("WeChat daily history"));
        assert!(md.contains("| Time |"));
        assert!(md.contains("Alice"));
    }

    #[test]
    fn table_shows_file_attachment_summary() {
        let query = WechatHistoryQuery::default();
        let tz: Tz = "Asia/Shanghai".parse().unwrap();
        let msg = WechatChatMessage {
            id: None,
            timestamp_ms: 1_718_800_000_000,
            conversation_id: "g@chatroom".into(),
            conversation_name: Some("Sales".into()),
            sender: Some("Bob".into()),
            sender_id: Some("wxid_b".into()),
            direction: MessageDirection::Inbound,
            msg_type: "app".into(),
            content: "订单.xlsx".into(),
            media_path: None,
            attachments: vec![WechatAttachment {
                kind: AttachmentKind::File,
                file_name: Some("订单.xlsx".into()),
                extension: Some("xlsx".into()),
                mime: None,
                size_bytes: Some(1024),
                sender_id: Some("wxid_b".into()),
                sender_name: Some("Bob".into()),
                conversation_id: "g@chatroom".into(),
                conversation_name: Some("Sales".into()),
                local_path: None,
                sha256: None,
                parse_status: AttachmentParseStatus::MetadataOnly,
                parsed: None,
                attach_id: None,
                error: None,
            }],
        };
        let md = render_markdown_table(&[msg], &query, tz);
        assert!(md.contains("订单.xlsx"));
        assert!(md.contains("[file:"));
    }
}
