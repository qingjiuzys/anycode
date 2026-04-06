//! FileRead 摘录：与压缩后注入共享的数据结构（`state` / `post_compact` 共用，避免模块环依赖）。

use anycode_core::prelude::*;
use anycode_tools::catalog::TOOL_FILE_READ;

pub const POST_COMPACT_MAX_FILES: usize = 5;
pub const POST_COMPACT_MAX_CHARS_PER_FILE: usize = 5_000;

#[derive(Debug, Clone)]
pub struct FileReadSnippet {
    pub path: String,
    pub excerpt: String,
}

fn try_parse_file_read_excerpt(raw: &str) -> Option<(String, String)> {
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    let path = v.get("path")?.as_str()?.to_string();
    let content = v.get("content")?.as_str()?;
    if content.is_empty() {
        return None;
    }
    let excerpt = if content.chars().count() > POST_COMPACT_MAX_CHARS_PER_FILE {
        content
            .chars()
            .take(POST_COMPACT_MAX_CHARS_PER_FILE)
            .collect::<String>()
            + "\n… [truncated]"
    } else {
        content.to_string()
    };
    Some((path, excerpt))
}

/// 自会话中收集最近若干次 FileRead 成功结果（同路径后者覆盖前者）。
pub fn collect_from_session(session: &[Message]) -> Vec<FileReadSnippet> {
    collect_from_session_with_max(session, POST_COMPACT_MAX_FILES)
}

/// 与 [`collect_from_session`] 相同，但可指定保留的最大文件数。
pub fn collect_from_session_with_max(
    session: &[Message],
    max_files: usize,
) -> Vec<FileReadSnippet> {
    let mut by_path: Vec<FileReadSnippet> = Vec::new();
    for msg in session {
        if msg.role != MessageRole::Tool {
            continue;
        }
        let name = msg
            .metadata
            .get("tool_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if name != TOOL_FILE_READ {
            continue;
        }
        let MessageContent::ToolResult {
            content, is_error, ..
        } = &msg.content
        else {
            continue;
        };
        if *is_error {
            continue;
        }
        if let Some((p, ex)) = try_parse_file_read_excerpt(content) {
            by_path.retain(|s| s.path != p);
            by_path.push(FileReadSnippet {
                path: p,
                excerpt: ex,
            });
        }
    }
    let len = by_path.len();
    if len > max_files {
        by_path.split_off(len - max_files)
    } else {
        by_path
    }
}
