#![allow(dead_code)]

//! 流式 Inline REPL：与全屏 TUI 共用 `message_to_entries` + `apply_tool_transcript_pipeline` + `layout_workspace`，再转为纯文本。

use anycode_core::Message;
use ratatui::text::Line;
use std::collections::HashSet;

use super::workspace_draw::layout_workspace;
use super::{
    apply_tool_transcript_pipeline, message_to_entries, TranscriptEntry, WorkspaceLiveLayout,
};

pub(crate) fn lines_to_plain_string(lines: &[Line<'static>]) -> String {
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        for span in &line.spans {
            out.push_str(span.content.as_ref());
        }
    }
    out
}

/// `exec_prev_len`：与 [`crate::tasks::repl_line_session::append_user_spawn_turn`] 返回值一致；
/// 取 `messages[exec_prev_len - 1..]` 以包含本轮用户句。
pub(crate) fn build_stream_turn_plain(
    exec_prev_len: usize,
    messages: &[Message],
    content_width: usize,
    executing: bool,
) -> String {
    let start = exec_prev_len.saturating_sub(1);
    let slice = messages.get(start..).unwrap_or(&[]);
    let mut entries: Vec<TranscriptEntry> = Vec::new();
    for m in slice {
        entries.extend(message_to_entries(m));
    }
    let mut fold_id = 0u64;
    apply_tool_transcript_pipeline(&mut entries, &mut fold_id);
    let expanded: HashSet<u64> = HashSet::new();
    let live = WorkspaceLiveLayout {
        executing,
        stream_repl_claude_user_prefix: true,
        ..Default::default()
    };
    let w = content_width.max(8);
    let lines = layout_workspace(&entries, w, &expanded, live);
    lines_to_plain_string(&lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_core::{MessageContent, MessageRole};
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    #[test]
    fn stream_plain_includes_user_claude_prefix_and_grows_with_assistant() {
        let exec_prev = 1usize;
        let mut msgs = vec![Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text("ping".into()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }];
        msgs.push(Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text("analyze".into()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        });
        let p1 = build_stream_turn_plain(exec_prev, &msgs, 80, false);
        assert!(
            p1.contains("❯") && p1.contains("analyze"),
            "expected Claude user prefix + prompt, got {p1:?}"
        );

        msgs.push(Message {
            id: Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: MessageContent::Text("hel".into()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        });
        let p2 = build_stream_turn_plain(exec_prev, &msgs, 80, false);
        assert!(
            p2.contains("hel"),
            "expected streaming assistant fragment, got {p2:?}"
        );
        if let MessageContent::Text(t) = &mut msgs[2].content {
            *t = "hello".into();
        }
        let p3 = build_stream_turn_plain(exec_prev, &msgs, 80, false);
        assert!(
            p3.contains("hello") && p3.len() > p2.len(),
            "expected grown assistant text, got p2={p2:?} p3={p3:?}"
        );
    }

    #[test]
    fn stream_executing_omits_last_user_until_turn_has_non_user_tail() {
        let exec_prev = 1usize;
        let msgs = vec![Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text("only-user".into()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }];
        let waiting = build_stream_turn_plain(exec_prev, &msgs, 80, true);
        assert!(
            !waiting.contains("only-user"),
            "expected empty main pane while waiting, got {waiting:?}"
        );
        let idle = build_stream_turn_plain(exec_prev, &msgs, 80, false);
        assert!(
            idle.contains("only-user"),
            "expected user line after turn layout, got {idle:?}"
        );
    }
}
