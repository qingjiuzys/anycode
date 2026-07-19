//! Task-local chat turn context (dashboard session / user turn / reply language).
//!
//! Embedded chat and UI triggers used to smuggle these values through process
//! environment variables (`ANYCODE_DASHBOARD_SESSION_ID`, `ANYCODE_REPLY_LANG`,
//! `ANYCODE_DASHBOARD_USER_TURN_ID`), which breaks isolation as soon as two
//! sessions run concurrently. This module scopes the values to the tokio task
//! that executes the turn. Environment variables remain only as a legacy
//! fallback for headless/CLI invocations that run one task per process.
//!
//! **Note:** mainstream chat/completions APIs expose no native `language` parameter;
//! `reply_language` is consumed by the agent system prompt and per-turn ephemeral
//! reminders (`crates/agent/src/reply_language.rs`), not by the LLM transport layer.

use serde::{Deserialize, Serialize};
use std::future::Future;

/// Structured per-turn context carried by [`crate::TaskContext::chat_turn`]
/// and scoped task-locally for the duration of a turn.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatTurnContext {
    /// Dashboard `sessions.id` this turn belongs to (approval/question scope).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dashboard_session_id: Option<String>,
    /// Monotonic user message id within the session (SSE scope key).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_turn_id: Option<u32>,
    /// Normalized reply language (`zh` / `en`) for the system prompt directive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_language: Option<String>,
    /// Optional host intent (e.g. start local server) appended to per-turn ephemeral reminder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_intent_hint: Option<String>,
}

tokio::task_local! {
    static CHAT_TURN: ChatTurnContext;
}

/// Run `fut` with `ctx` visible via the `current_*` accessors on this task.
pub async fn scope_chat_turn<F: Future>(ctx: ChatTurnContext, fut: F) -> F::Output {
    CHAT_TURN.scope(ctx, fut).await
}

/// Full context for the current task, when scoped.
#[must_use]
pub fn current_chat_turn() -> Option<ChatTurnContext> {
    CHAT_TURN.try_with(Clone::clone).ok()
}

/// Dashboard session id for the current task, when scoped.
#[must_use]
pub fn current_dashboard_session_id() -> Option<String> {
    CHAT_TURN
        .try_with(|c| c.dashboard_session_id.clone())
        .ok()
        .flatten()
}

/// User turn id for the current task, when scoped.
#[must_use]
pub fn current_user_turn_id() -> Option<u32> {
    CHAT_TURN.try_with(|c| c.user_turn_id).ok().flatten()
}

/// Reply language for the current task, when scoped.
#[must_use]
pub fn current_reply_language() -> Option<String> {
    CHAT_TURN
        .try_with(|c| c.reply_language.clone())
        .ok()
        .flatten()
}

/// Host intent hint for the current task, when scoped.
#[must_use]
pub fn current_host_intent_hint() -> Option<String> {
    CHAT_TURN
        .try_with(|c| c.host_intent_hint.clone())
        .ok()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn scoped_context_visible_only_inside_scope() {
        assert!(current_chat_turn().is_none());
        let ctx = ChatTurnContext {
            dashboard_session_id: Some("sess_a".into()),
            user_turn_id: Some(3),
            reply_language: Some("zh".into()),
            host_intent_hint: None,
        };
        scope_chat_turn(ctx.clone(), async {
            assert_eq!(current_dashboard_session_id().as_deref(), Some("sess_a"));
            assert_eq!(current_user_turn_id(), Some(3));
            assert_eq!(current_reply_language().as_deref(), Some("zh"));
            assert_eq!(current_chat_turn(), Some(ctx));
        })
        .await;
        assert!(current_chat_turn().is_none());
    }

    #[tokio::test]
    async fn concurrent_scopes_do_not_leak_between_tasks() {
        let a = tokio::spawn(scope_chat_turn(
            ChatTurnContext {
                dashboard_session_id: Some("sess_a".into()),
                user_turn_id: Some(1),
                reply_language: Some("zh".into()),
                host_intent_hint: None,
            },
            async {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                current_dashboard_session_id()
            },
        ));
        let b = tokio::spawn(scope_chat_turn(
            ChatTurnContext {
                dashboard_session_id: Some("sess_b".into()),
                user_turn_id: Some(9),
                reply_language: Some("en".into()),
                host_intent_hint: None,
            },
            async {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                current_dashboard_session_id()
            },
        ));
        assert_eq!(a.await.unwrap().as_deref(), Some("sess_a"));
        assert_eq!(b.await.unwrap().as_deref(), Some("sess_b"));
    }
}
