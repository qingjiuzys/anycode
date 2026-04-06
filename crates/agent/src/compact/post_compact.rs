//! 与 Claude Code `createPostCompactFileAttachments` 意图对齐：压缩后注入**少量**最近 FileRead 摘录，避免摘要丢光关键文件内容。
//!
//! anyCode 无 `readFileState` 缓存，从**压缩前**会话里的 FileRead `tool_result` JSON 解析 `path` + `content`。

use crate::compact::state::SessionCompactionState;
use anycode_core::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

use super::snippets::{self, FileReadSnippet};

const META_POST_COMPACT_FILES: &str = "anycode_post_compact_files";

/// 自 `pre_compact_session` 中收集最近若干次 FileRead 成功结果（同路径后者覆盖前者）。
#[allow(dead_code)] // 对外 API；默认压缩路径经 `SessionCompactionState` / hooks
pub fn collect_file_read_snippets(
    pre_compact_session: &[Message],
    max_files: usize,
) -> Vec<(String, String)> {
    snippets::collect_from_session_with_max(pre_compact_session, max_files)
        .into_iter()
        .map(|s| (s.path, s.excerpt))
        .collect()
}

fn inject_snippets_into(out: &mut Vec<Message>, snippets: &[FileReadSnippet]) {
    if snippets.is_empty() {
        return;
    }

    let mut body = String::from(
        "## Context from recent file reads (before compaction)\n\n\
         The following excerpts were preserved from FileRead tool results. \
         Use them if you need exact text from before the summary.\n\n",
    );
    for s in snippets {
        body.push_str("### ");
        body.push_str(&s.path);
        body.push_str("\n\n```\n");
        body.push_str(&s.excerpt);
        body.push_str("\n```\n\n");
    }

    let mut meta = HashMap::new();
    meta.insert(
        META_POST_COMPACT_FILES.to_string(),
        serde_json::Value::Bool(true),
    );
    out.push(Message {
        id: Uuid::new_v4(),
        role: MessageRole::User,
        content: MessageContent::Text(body),
        timestamp: chrono::Utc::now(),
        metadata: meta,
    });
}

/// 在 `[system, summary_user, …]` 末尾追加一条 user，携带 `state` 中的文件摘录（若存在）。
pub fn inject_file_snippets_from_state(out: &mut Vec<Message>, state: &SessionCompactionState) {
    inject_snippets_into(out, &state.file_reads);
}

/// 在 `[system, summary_user, …]` 末尾追加一条 user，携带文件摘录（若存在）。
#[allow(dead_code)] // 对外 API；`cargo build --lib` 无 `#[cfg(test)]` 调用方时仍会报未使用
pub fn inject_file_read_snippets(out: &mut Vec<Message>, pre_compact_session: &[Message]) {
    let snippets = snippets::collect_from_session(pre_compact_session);
    inject_snippets_into(out, &snippets);
}

/// 与 Claude `runPostCompactCleanup` 对齐的占位：释放/重置压缩失效的全局缓存。
/// anyCode 当前无 `getUserContext` / session 级缓存，仅打调试日志供后续扩展。
pub fn run_post_compact_cleanup() {
    tracing::debug!(target: "anycode_agent", "post_compact_cleanup");
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_tools::catalog::TOOL_FILE_READ;
    use std::collections::HashMap;
    use uuid::Uuid;

    #[test]
    fn inject_adds_user_when_file_read_present() {
        let path = "/tmp/x.rs";
        let json = serde_json::json!({
            "path": path,
            "content": "fn main() {}",
            "encoding": "utf-8"
        })
        .to_string();
        let mut meta = HashMap::new();
        meta.insert(
            "tool_name".to_string(),
            serde_json::Value::String(TOOL_FILE_READ.to_string()),
        );
        let pre = vec![Message {
            id: Uuid::new_v4(),
            role: MessageRole::Tool,
            content: MessageContent::ToolResult {
                tool_use_id: "t1".into(),
                content: json,
                is_error: false,
            },
            timestamp: chrono::Utc::now(),
            metadata: meta,
        }];
        let mut out = vec![Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text("summary".into()),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }];
        inject_file_read_snippets(&mut out, &pre);
        assert_eq!(out.len(), 2);
        let t = match &out[1].content {
            MessageContent::Text(s) => s.as_str(),
            _ => panic!("expected text"),
        };
        assert!(t.contains("Context from recent file reads"));
        assert!(t.contains(path));
        assert!(t.contains("fn main()"));
    }
}
