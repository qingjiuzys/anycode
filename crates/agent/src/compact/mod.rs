//! 会话压缩（与 Claude Code `services/compact` 对齐）：无工具摘要调用 + `formatCompactSummary` + 续接 user 摘要。
//!
//! - **microcompact**：全量压缩前清空较早可压缩工具的 `tool_result`（与 Claude time-based MC 占位符一致）。
//! - **post_compact**：从压缩前会话恢复最近 FileRead 摘录（对齐 `createPostCompactFileAttachments` 意图）。

#![allow(unused_imports)] // barrel `pub use`，供 `crate::compact::` / `anycode_agent::` 与 `hooks` 子模块使用

mod hooks;
mod microcompact;
mod policy;
mod post_compact;
mod snippets;
mod state;

pub use hooks::{
    CompactionHooks, CompactionPostContext, CompactionPreContext, DefaultCompactionHooks,
};
pub use microcompact::{apply_microcompact, default_keep_recent};
pub use policy::CompactPolicy;
pub use post_compact::{
    inject_file_read_snippets, inject_file_snippets_from_state, run_post_compact_cleanup,
};
pub use snippets::{POST_COMPACT_MAX_CHARS_PER_FILE, POST_COMPACT_MAX_FILES};
pub use state::{FileReadSnippet, SessionCompactionState};

use anycode_core::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;
use uuid::Uuid;

/// Claude Code `PROMPT_TOO_LONG_ERROR_MESSAGE`（摘要响应内嵌重试信号）。
pub const PROMPT_TOO_LONG_PREFIX: &str = "Prompt is too long";

const NO_TOOLS_PREAMBLE: &str = "CRITICAL: Respond with TEXT ONLY. Do NOT call any tools.

- Do NOT use Read, Bash, Grep, Glob, Edit, Write, or ANY other tool.
- You already have all the context you need in the conversation above.
- Tool calls will be REJECTED and will waste your only turn — you will fail the task.
- Your entire response must be plain text: an <analysis> block followed by a <summary> block.

";

const NO_TOOLS_TRAILER: &str =
    "\n\nREMINDER: Do NOT call any tools. Respond with plain text only — \
an <analysis> block followed by a <summary> block. \
Tool calls will be rejected and you will fail the task.";

const BASE_COMPACT_BODY: &str = include_str!("base_compact_body.txt");

const MAX_COMPACT_PTL_RETRIES: usize = 3;

/// 摘要模型 `max_tokens` 上限（与 Claude `COMPACT_MAX_OUTPUT_TOKENS` 同量级）。
pub const COMPACT_MAX_OUTPUT_TOKENS: u32 = 20_000;

static RE_ANALYSIS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<analysis>.*?</analysis>").expect("analysis regex"));
static RE_SUMMARY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<summary>(.*?)</summary>").expect("summary regex"));

/// 与 Claude `formatCompactSummary` 一致。
pub fn format_compact_summary(summary: &str) -> String {
    let mut formatted = RE_ANALYSIS.replace_all(summary, "").to_string();
    if let Some(caps) = RE_SUMMARY.captures(&formatted) {
        let content = caps.get(1).map(|m| m.as_str()).unwrap_or("").trim();
        formatted = RE_SUMMARY
            .replace(&formatted, format!("Summary:\n{content}"))
            .to_string();
    }
    let re_nl = Regex::new(r"\n\n+").expect("nl regex");
    re_nl.replace_all(formatted.trim(), "\n\n").to_string()
}

fn get_compact_prompt(custom_instructions: Option<&str>) -> String {
    let mut prompt = format!("{NO_TOOLS_PREAMBLE}{BASE_COMPACT_BODY}");
    if let Some(ci) = custom_instructions {
        let t = ci.trim();
        if !t.is_empty() {
            prompt.push_str("\n\nAdditional Instructions:\n");
            prompt.push_str(t);
        }
    }
    prompt.push_str(NO_TOOLS_TRAILER);
    prompt
}

/// 与 Claude `getCompactUserSummaryMessage` 对齐（无 transcript 路径、无 proactive 分支）。
pub fn get_compact_user_summary_message(
    formatted_summary: &str,
    suppress_follow_up_questions: bool,
    transcript_path: Option<&str>,
) -> String {
    let mut base = format!(
        "This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.\n\n{formatted_summary}"
    );
    if let Some(p) = transcript_path {
        if !p.trim().is_empty() {
            base.push_str(&format!(
                "\n\nIf you need specific details from before compaction (like exact code snippets, error messages, or content you generated), read the full transcript at: {p}"
            ));
        }
    }
    if suppress_follow_up_questions {
        base.push_str(
            "\n\nContinue the conversation from where it left off without asking the user any further questions. Resume directly — do not acknowledge the summary, do not recap what was happening, do not preface with \"I'll continue\" or similar. Pick up the last task as if the break never happened.",
        );
    }
    base
}

/// 待送入摘要 API 的起始下标：含「最后一条 compact 摘要 user」本身（与 Claude `getMessagesAfterCompactBoundary` 一致）；无则 `1`（跳过 system）。
pub fn summarization_start_index(msgs: &[Message]) -> usize {
    for (i, m) in msgs.iter().enumerate().rev() {
        if m.role != MessageRole::User {
            continue;
        }
        if m.metadata
            .get(ANYCODE_COMPACT_SUMMARY_METADATA_KEY)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return i;
        }
    }
    1
}

/// `[system] + 自上次压缩摘要起的尾部`，用于摘要 API。
pub fn build_compact_api_messages(
    fresh_system: Message,
    session: &[Message],
) -> Result<Vec<Message>, CoreError> {
    if session.is_empty() {
        return Err(CoreError::LLMError("compact: empty session".into()));
    }
    if session.first().map(|m| &m.role) != Some(&MessageRole::System) {
        return Err(CoreError::LLMError(
            "compact: first message must be system".into(),
        ));
    }
    let start = summarization_start_index(session);
    if start >= session.len() {
        return Err(CoreError::LLMError(
            "compact: no messages to summarize".into(),
        ));
    }
    let mut out = vec![fresh_system];
    out.extend(session[start..].iter().cloned());
    Ok(out)
}

fn truncate_head_after_system(messages: &mut Vec<Message>) -> bool {
    if messages.len() <= 2 {
        return false;
    }
    messages.remove(1);
    true
}

fn assistant_plain(response: &LLMResponse) -> String {
    match &response.message.content {
        MessageContent::Text(t) => t.clone(),
        _ => String::new(),
    }
}

/// 调用摘要模型；`messages` 须已含 system + 待摘要历史；末尾由本函数追加 compact user prompt。
pub async fn run_compact_llm(
    llm: &std::sync::Arc<dyn LLMClient>,
    summary_model: &ModelConfig,
    mut messages: Vec<Message>,
    custom_instructions: Option<&str>,
) -> Result<(String, Usage), CoreError> {
    let compact_user = Message {
        id: Uuid::new_v4(),
        role: MessageRole::User,
        content: MessageContent::Text(get_compact_prompt(custom_instructions)),
        timestamp: chrono::Utc::now(),
        metadata: HashMap::new(),
    };

    let mut model = summary_model.clone();
    model.max_tokens = Some(COMPACT_MAX_OUTPUT_TOKENS);

    let mut ptl_attempts = 0usize;
    loop {
        let mut req = messages.clone();
        req.push(compact_user.clone());

        let response = llm.chat(req, vec![], &model).await?;
        if !response.tool_calls.is_empty() {
            return Err(CoreError::LLMError(
                "compact: model returned tool calls; expected text only".into(),
            ));
        }

        let text = assistant_plain(&response);
        let usage = response.usage;

        if text.trim_start().starts_with(PROMPT_TOO_LONG_PREFIX) {
            ptl_attempts += 1;
            if ptl_attempts > MAX_COMPACT_PTL_RETRIES || !truncate_head_after_system(&mut messages)
            {
                return Err(CoreError::LLMError(
                    "compact: prompt too long after retries".into(),
                ));
            }
            continue;
        }

        if text.trim().is_empty() {
            return Err(CoreError::LLMError(
                "compact: empty assistant response".into(),
            ));
        }

        return Ok((text, usage));
    }
}

/// 压缩后写回：`[fresh_system, summary_user]`。
pub fn build_post_compact_messages(
    fresh_system: Message,
    raw_assistant_summary: &str,
    suppress_follow_up: bool,
    transcript_path: Option<&str>,
) -> Result<Vec<Message>, CoreError> {
    let formatted = format_compact_summary(raw_assistant_summary);
    if formatted.trim().is_empty() {
        return Err(CoreError::LLMError(
            "compact: formatted summary empty".into(),
        ));
    }
    let body = get_compact_user_summary_message(&formatted, suppress_follow_up, transcript_path);
    let mut meta = HashMap::new();
    meta.insert(
        ANYCODE_COMPACT_SUMMARY_METADATA_KEY.to_string(),
        serde_json::Value::Bool(true),
    );
    Ok(vec![
        fresh_system,
        Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text(body),
            timestamp: chrono::Utc::now(),
            metadata: meta,
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_too_long_prefix_matches_claude_compact_contract() {
        assert_eq!(PROMPT_TOO_LONG_PREFIX, "Prompt is too long");
    }

    #[test]
    fn format_strips_analysis_and_expands_summary() {
        let raw = r#"<analysis>
x
</analysis>

<summary>
1. A:
   hi
</summary>"#;
        let f = format_compact_summary(raw);
        assert!(!f.contains("<analysis>"));
        assert!(f.contains("Summary:"));
        assert!(f.contains("1. A:"));
    }
}
