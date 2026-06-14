//! Read-only local WeChat chat history query for anyCode agents.

mod backend;
mod config;
mod date_filter;
mod error;
mod file_parser;
mod format;
mod model;
mod name_resolver;
mod runtime;
mod wc4_appmsg;

pub use backend::{build_backend, WechatHistoryBackend};
pub use config::{WechatHistoryBackendKind, WechatHistoryConfig};
pub use error::{Result, WechatHistoryError};
pub use model::{
    MessageDirection, WechatChatMessage, WechatConversation, WechatHistoryOutputFormat,
    WechatHistoryQuery, WechatHistoryResult,
};

use format::render_markdown_table;

/// Query local WeChat chat history for a calendar day.
pub fn query_history(
    config: &WechatHistoryConfig,
    query: &WechatHistoryQuery,
    format: WechatHistoryOutputFormat,
) -> Result<WechatHistoryResult> {
    let backend = build_backend(config)?;
    let mut result = backend.query(query, config)?;
    if format == WechatHistoryOutputFormat::MarkdownTable {
        if result.markdown_table.is_none() {
            let (_, tz, _, _) = date_filter::validate_query(query)?;
            result.markdown_table = Some(render_markdown_table(&result.messages, query, tz));
        }
    }
    Ok(result)
}
