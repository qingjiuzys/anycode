//! 任务收尾：直接 assistant 正文 vs LLM 总结回执。

use anycode_core::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

/// 规整模型产出的 Markdown 总结：去掉汉字间误插的空格、整理 `**` 内空白、合并重复的 `###` 小节标题。
/// 用于终端回执与 `notifications` HTTP `excerpt` 等展示，避免模型偶发「字间空格 / 重复小标题」。
pub(crate) fn normalize_task_summary_markdown(raw: String) -> String {
    let s = remove_spaces_between_cjk_ideographs(&raw);
    let s = trim_markdown_bold_inners(&s);
    dedupe_h3_blocks_keep_last(&s)
}

fn is_cjk_ideograph_for_normalize(c: char) -> bool {
    matches!(
        c as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x3005..=0x3007
    )
}

fn is_space_between_cjk(a: char, mid: char, b: char) -> bool {
    is_cjk_ideograph_for_normalize(a)
        && mid.is_ascii_whitespace()
        && is_cjk_ideograph_for_normalize(b)
}

fn remove_spaces_between_cjk_ideographs(s: &str) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = 0;
        while i + 2 < chars.len() {
            if is_space_between_cjk(chars[i], chars[i + 1], chars[i + 2]) {
                chars.remove(i + 1);
                changed = true;
            } else {
                i += 1;
            }
        }
    }
    chars.into_iter().collect()
}

fn trim_markdown_bold_inners(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("**") {
        out.push_str(&rest[..start]);
        rest = &rest[start + 2..];
        match rest.find("**") {
            Some(end) => {
                let inner = rest[..end].trim();
                out.push_str("**");
                out.push_str(inner);
                out.push_str("**");
                rest = &rest[end + 2..];
            }
            None => {
                out.push_str("**");
                out.push_str(rest);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

fn is_markdown_h3_heading(line: &str) -> bool {
    let s = line.trim_start();
    if s.len() < 4 || !s.starts_with("###") {
        return false;
    }
    s[3..].chars().next().is_some_and(|c| c.is_whitespace())
}

fn h3_title_key(line: &str) -> String {
    let s = line.trim_start();
    let after_hashes = s.strip_prefix("###").map(str::trim_start).unwrap_or("");
    after_hashes
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn dedupe_h3_blocks_keep_last(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let mut i = 0;
    let mut preamble: Vec<String> = Vec::new();
    let mut blocks: Vec<(String, String, Vec<String>)> = Vec::new();

    while i < lines.len() {
        let line = lines[i];
        if is_markdown_h3_heading(line) {
            let title_line = line.to_string();
            let key = h3_title_key(&title_line);
            i += 1;
            let mut body = Vec::new();
            while i < lines.len() && !is_markdown_h3_heading(lines[i]) {
                body.push(lines[i].to_string());
                i += 1;
            }
            blocks.push((key, title_line, body));
        } else {
            preamble.push(line.to_string());
            i += 1;
        }
    }

    let mut last_by_key: HashMap<String, usize> = HashMap::new();
    for (idx, (key, _, _)) in blocks.iter().enumerate() {
        last_by_key.insert(key.clone(), idx);
    }

    let mut out = String::new();
    if !preamble.is_empty() {
        out.push_str(&preamble.join("\n"));
    }
    for (idx, (key, title_line, body)) in blocks.iter().enumerate() {
        if last_by_key.get(key) != Some(&idx) {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(title_line);
        for line in body {
            out.push('\n');
            out.push_str(line);
        }
    }
    out.trim_end().to_string()
}

/// 若末条为带正文的 assistant，返回该正文（trim 后非空）。
pub(crate) fn last_assistant_plain_text(messages: &[Message]) -> Option<String> {
    if let Some(last) = messages.last() {
        if last.role == MessageRole::Assistant {
            if let MessageContent::Text(t) = &last.content {
                let fast = t.trim();
                if !fast.is_empty() {
                    return Some(fast.to_string());
                }
            }
        }
    }
    None
}

/// 基于运行轨迹生成中文 summary（失败时返回可展示的错误占位字符串，不向上 Err）。
pub(crate) async fn llm_summary_receipt(
    llm_client: &Arc<dyn LLMClient>,
    summary_model: &ModelConfig,
    task: &Task,
    total_tool_calls: usize,
    max_turns: usize,
    max_tool_calls: usize,
    artifacts_brief: &str,
    output_tail: &str,
) -> String {
    let summary_messages = vec![
        Message {
            id: uuid::Uuid::new_v4(),
            role: MessageRole::System,
            content: MessageContent::Text(
                "你是 anyCode 的运行总结器。请基于给定的信息，为用户输出 5-10 行中文“完成回执”总结：做了什么、关键步骤（含工具调用概况）、结果/下一步。不要输出冗长正文，不要复述原始日志。\n\
                版式要求：使用规范 Markdown；加粗必须写成 `**词语**`，`**` 与文字之间不要空格；不要在连续汉字之间插入空格（英文单词之间照常保留空格）。若使用 `###` 小标题，编号连续且同一标题只出现一次，不要重复粘贴上一节内容。".to_string()
            ),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        },
        Message {
            id: uuid::Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text(format!(
                "agent_type: {}\nprompt: {}\nturns_max: {}\ntool_calls_max: {}\ntotal_tool_calls: {}\n\n== artifacts ==\n{}\n\n== run_log_tail ==\n{}",
                task.agent_type.as_str(),
                task.prompt,
                max_turns,
                max_tool_calls,
                total_tool_calls,
                artifacts_brief,
                output_tail
            )),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        },
    ];

    match llm_client
        .chat(summary_messages, vec![], summary_model)
        .await
    {
        Ok(summary_resp) => match summary_resp.message.content {
            MessageContent::Text(t) => normalize_task_summary_markdown(t),
            _ => "（summary 生成失败：unexpected content）".to_string(),
        },
        Err(e) => format!("（summary 生成失败：{}）", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_removes_spaces_between_cjk() {
        assert_eq!(
            normalize_task_summary_markdown("### 2. 关 键 技 术 特 性\n- a".to_string()),
            "### 2. 关键技术特性\n- a"
        );
    }

    #[test]
    fn normalize_trims_bold_inner_whitespace() {
        assert_eq!(
            normalize_task_summary_markdown("**多 通 道 桥 接 **：说明".to_string()),
            "**多通道桥接**：说明"
        );
    }

    #[test]
    fn normalize_keeps_space_between_ascii_and_cjk() {
        assert_eq!(
            normalize_task_summary_markdown("**MCP 支 持**".to_string()),
            "**MCP 支持**"
        );
    }

    #[test]
    fn normalize_dedupes_duplicate_h3_title_keep_last() {
        let raw = "### 1. A\nx\n### 3. 依赖关系图\n- dup\n### 3. 依赖关系图\nok";
        assert_eq!(
            normalize_task_summary_markdown(raw.to_string()),
            "### 1. A\nx\n### 3. 依赖关系图\nok"
        );
    }

    #[test]
    fn last_assistant_detects_trailing_text() {
        let messages = vec![Message {
            id: uuid::Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: MessageContent::Text("  done  ".to_string()),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }];
        assert_eq!(
            last_assistant_plain_text(&messages).as_deref(),
            Some("done")
        );
    }

    #[test]
    fn last_assistant_ignores_empty_tail() {
        let messages = vec![Message {
            id: uuid::Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: MessageContent::Text("   \n  ".to_string()),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }];
        assert!(last_assistant_plain_text(&messages).is_none());
    }
}
