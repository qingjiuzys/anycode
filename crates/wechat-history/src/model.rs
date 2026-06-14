use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageDirection {
    Inbound,
    Outbound,
    Unknown,
}

impl MessageDirection {
    pub fn as_label(&self) -> &'static str {
        match self {
            Self::Inbound => "inbound",
            Self::Outbound => "outbound",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentKind {
    File,
    Image,
    Voice,
    Video,
    Link,
    MiniProgram,
    #[default]
    Unknown,
}

impl AttachmentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Image => "image",
            Self::Voice => "voice",
            Self::Video => "video",
            Self::Link => "link",
            Self::MiniProgram => "mini_program",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentParseStatus {
    NotApplicable,
    Parsed,
    Unsupported,
    MissingFile,
    ParseFailed,
    MetadataOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcelColumnHint {
    pub name: String,
    pub column_index: usize,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcelSummary {
    pub sheet_names: Vec<String>,
    pub active_sheet: String,
    pub row_count: usize,
    pub column_count: usize,
    pub header_row: Vec<String>,
    pub preview_rows: Vec<Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub detected_fields: Vec<ExcelColumnHint>,
    #[serde(default)]
    pub pii_redacted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextFileSummary {
    pub line_count: usize,
    pub preview_lines: Vec<String>,
    #[serde(default)]
    pub pii_redacted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ParsedFilePreview {
    Excel(ExcelSummary),
    Text(TextFileSummary),
    Json(JsonValue),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatAttachment {
    pub kind: AttachmentKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_name: Option<String>,
    pub conversation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    pub parse_status: AttachmentParseStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parsed: Option<ParsedFilePreview>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attach_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatConversation {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub is_group: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatChatMessage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub timestamp_ms: i64,
    pub conversation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_id: Option<String>,
    pub direction: MessageDirection,
    pub msg_type: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<WechatAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatHistoryQuery {
    /// Local calendar date `YYYY-MM-DD`.
    pub date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(default)]
    pub include_group_sender: bool,
    /// Include attachment metadata on file/app messages (default true).
    #[serde(default = "default_true")]
    pub include_attachments: bool,
    /// Parse local file bytes for supported types (Excel, text). Default false.
    #[serde(default)]
    pub parse_files: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_types: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_file_parse_bytes: Option<u64>,
}

fn default_true() -> bool {
    true
}

impl Default for WechatHistoryQuery {
    fn default() -> Self {
        Self {
            date: String::new(),
            conversation: None,
            keyword: None,
            timezone: None,
            limit: None,
            include_group_sender: false,
            include_attachments: true,
            parse_files: false,
            attachment_types: None,
            max_file_parse_bytes: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WechatHistoryOutputFormat {
    #[default]
    Records,
    MarkdownTable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentStats {
    pub attachment_messages: usize,
    pub resolved_files: usize,
    pub parsed_files: usize,
    pub parse_failures: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatHistoryResult {
    pub date: String,
    pub timezone: String,
    pub backend: String,
    pub total: usize,
    pub truncated: bool,
    pub messages: Vec<WechatChatMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markdown_table: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_stats: Option<AttachmentStats>,
}
