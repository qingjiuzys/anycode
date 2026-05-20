//! Shared helpers for multi-turn LLM + tool agentic loops (`execute_task` / `execute_turn_from_messages`).

use anycode_core::prelude::*;
use std::future::pending;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Short-interval polling so `tokio::select!` can abort in-flight LLM I/O without `tokio-util`.
pub(super) const COOP_CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(20);

pub(super) fn nested_coop_cancelled(ctx: &TaskContext) -> bool {
    ctx.nested_cancel
        .as_ref()
        .is_some_and(|b| b.load(Ordering::Acquire))
}

pub(super) fn opt_coop_cancelled(flag: &Option<Arc<AtomicBool>>) -> bool {
    flag.as_ref().is_some_and(|b| b.load(Ordering::Acquire))
}

pub(super) fn task_cancelled_failure() -> TaskResult {
    TaskResult::Failure {
        error: NESTED_TASK_COOPERATIVE_CANCEL_ERROR.to_string(),
        details: Some("cooperative nested cancel".to_string()),
    }
}

pub(super) async fn coop_flag_wait(flag: Arc<AtomicBool>) {
    loop {
        if flag.load(Ordering::Acquire) {
            return;
        }
        tokio::time::sleep(COOP_CANCEL_POLL_INTERVAL).await;
    }
}

/// For `select!`: when `flag` is `None`, this future never completes (LLM branch always runs).
pub(super) async fn coop_flag_wait_opt(flag: Option<Arc<AtomicBool>>) {
    match flag {
        Some(f) => coop_flag_wait(f).await,
        None => pending().await,
    }
}

pub(super) async fn pop_assistant_placeholder(
    messages: &Arc<Mutex<Vec<Message>>>,
    assistant_id: Uuid,
) {
    let mut g = messages.lock().await;
    if g.last().is_some_and(|m| m.id == assistant_id) {
        g.pop();
    }
}

pub(super) fn estimate_input_tokens_for_messages(messages: &[Message]) -> u32 {
    let chars: usize = messages
        .iter()
        .map(|m| match &m.content {
            MessageContent::Text(t) => t.chars().count(),
            MessageContent::ToolResult { content, .. } => content.chars().count(),
            _ => 0,
        })
        .sum();
    ((chars as u32).saturating_add(3)) / 4
}

/// Rehydrate a non-streaming `LLMResponse` after a successful stream (placeholder already in history).
pub(super) async fn rehydrate_stream_llm_response(
    messages: &Arc<Mutex<Vec<Message>>>,
    assistant_id: Uuid,
    tool_calls: Vec<ToolCall>,
    stream_usage: Option<Usage>,
    messages_snapshot: &[Message],
) -> LLMResponse {
    let assistant_msg = {
        let g = messages.lock().await;
        g.iter()
            .rev()
            .find(|m| m.id == assistant_id)
            .cloned()
            .unwrap_or(Message {
                id: assistant_id,
                role: MessageRole::Assistant,
                content: MessageContent::Text(String::new()),
                timestamp: chrono::Utc::now(),
                metadata: std::collections::HashMap::new(),
            })
    };
    LLMResponse {
        message: assistant_msg,
        tool_calls,
        usage: stream_usage.unwrap_or_else(|| Usage {
            input_tokens: estimate_input_tokens_for_messages(messages_snapshot),
            output_tokens: 0,
            cache_creation_tokens: None,
            cache_read_tokens: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    fn msg(role: MessageRole, id: Uuid, text: &str) -> Message {
        Message {
            id,
            role,
            content: MessageContent::Text(text.to_string()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn pop_assistant_placeholder_removes_tail_match() {
        let aid = Uuid::new_v4();
        let msgs = Arc::new(Mutex::new(vec![
            msg(MessageRole::User, Uuid::new_v4(), "hi"),
            msg(MessageRole::Assistant, aid, ""),
        ]));
        pop_assistant_placeholder(&msgs, aid).await;
        assert_eq!(msgs.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn pop_assistant_placeholder_skips_when_tail_id_differs() {
        let aid = Uuid::new_v4();
        let other = Uuid::new_v4();
        let msgs = Arc::new(Mutex::new(vec![
            msg(MessageRole::Assistant, aid, ""),
            msg(MessageRole::Assistant, other, "final"),
        ]));
        pop_assistant_placeholder(&msgs, aid).await;
        assert_eq!(msgs.lock().await.len(), 2);
    }

    #[test]
    fn estimate_input_tokens_uses_text_char_count() {
        let msgs = vec![msg(MessageRole::User, Uuid::new_v4(), "12345678")];
        assert_eq!(estimate_input_tokens_for_messages(&msgs), 2);
    }

    #[test]
    fn estimate_input_tokens_empty_messages_is_zero() {
        assert_eq!(estimate_input_tokens_for_messages(&[]), 0);
    }
}
