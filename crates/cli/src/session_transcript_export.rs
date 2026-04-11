//! 只读上下文展示与会话导出（`/context`、`/export`、`/cost`），与持久化消息同源。

use anycode_core::{Message, MessageContent, MessageRole, Usage};

pub(crate) fn format_context_lines(
    message_count: usize,
    context_window_tokens: u32,
    last_max_input_tokens: u32,
    last_turn_usage: Option<&Usage>,
) -> Vec<String> {
    let mut lines = vec![
        format!("messages: {message_count}"),
        format!("context_window_tokens: {context_window_tokens}"),
    ];
    if last_max_input_tokens > 0 {
        lines.push(format!(
            "last_turn_max_input_tokens: {last_max_input_tokens}"
        ));
    }
    if let Some(u) = last_turn_usage {
        lines.push(format!("last_turn_usage_input_tokens: {}", u.input_tokens));
        lines.push(format!(
            "last_turn_usage_output_tokens: {}",
            u.output_tokens
        ));
        if let Some(c) = u.cache_read_tokens {
            lines.push(format!("last_turn_cache_read_tokens: {c}"));
        }
        if let Some(c) = u.cache_creation_tokens {
            lines.push(format!("last_turn_cache_creation_tokens: {c}"));
        }
    } else if last_max_input_tokens == 0 {
        lines.push("last_turn_usage: (none yet)".to_string());
    }
    lines
}

/// `/cost`：与 [`format_context_lines`] 相同数据，前置「不提供货币计费」说明。
pub(crate) fn format_cost_lines(
    message_count: usize,
    context_window_tokens: u32,
    last_max_input_tokens: u32,
    last_turn_usage: Option<&Usage>,
) -> Vec<String> {
    let mut lines = vec![crate::i18n::tr("repl-cost-disclaimer")];
    lines.extend(format_context_lines(
        message_count,
        context_window_tokens,
        last_max_input_tokens,
        last_turn_usage,
    ));
    lines
}

fn push_role_text(out: &mut String, role: &MessageRole, text: &str) {
    let label = match role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
    };
    let t = text.trim_end();
    if t.is_empty() {
        return;
    }
    out.push_str(&format!("--- {label} ---\n{t}\n\n"));
}

/// 将当前会话消息导出为可读纯文本（非 JSON 快照）。
pub(crate) fn messages_to_plain_export(messages: &[Message]) -> String {
    let mut out = String::new();
    for m in messages {
        match &m.content {
            MessageContent::Text(t) => push_role_text(&mut out, &m.role, t),
            MessageContent::ToolUse { name, input } => {
                let body = format!(
                    "[tool use: {name}]\n{}",
                    serde_json::to_string_pretty(input).unwrap_or_else(|_| input.to_string())
                );
                push_role_text(&mut out, &m.role, &body);
            }
            MessageContent::ToolResult {
                content, is_error, ..
            } => {
                let pfx = if *is_error { "tool (error)" } else { "tool" };
                push_role_text(&mut out, &m.role, &format!("[{pfx}]\n{content}"));
            }
        }
    }
    out.trim_end().to_string()
}
