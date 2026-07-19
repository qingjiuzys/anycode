//! Per-turn reply language reminders for LLM requests.
//!
//! Chat/completions APIs have no native `language` / `locale` parameter; control
//! flows UI `lang` → [`anycode_core::scope_chat_turn`] → system prompt directive.
//! This module appends an **ephemeral** system reminder on each LLM call so tool-loop
//! turns do not dilute the directive.

use anycode_core::prelude::*;
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

pub(crate) const REPLY_LANGUAGE_REMINDER_METADATA: &str = "reply_language_reminder";

/// Short reminder text for the active reply language, if any.
/// Body lives under `prompts/locale/<tag>/ephemeral_reminder.md`.
#[must_use]
pub(crate) fn ephemeral_reply_language_reminder_text() -> Option<String> {
    crate::prompt_catalog::ephemeral_reminder_text()
}

/// Append an ephemeral system reminder to a **request-only** message snapshot.
///
/// Combines reply-language reminder with optional [`anycode_core::ChatTurnContext::host_intent_hint`]
/// so host-detected intents (e.g. start local server) survive tool-loop dilution.
#[must_use]
pub(crate) fn inject_ephemeral_reply_language_reminder(mut messages: Vec<Message>) -> Vec<Message> {
    let lang = ephemeral_reply_language_reminder_text();
    let intent = anycode_core::current_host_intent_hint();
    let text = match (lang, intent) {
        (Some(l), Some(i)) => format!("{l}\n\n{i}"),
        (Some(l), None) => l,
        (None, Some(i)) => i,
        (None, None) => return messages,
    };

    messages.retain(|m| {
        !m.metadata
            .get(REPLY_LANGUAGE_REMINDER_METADATA)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    });

    let mut metadata = HashMap::new();
    metadata.insert(
        REPLY_LANGUAGE_REMINDER_METADATA.to_string(),
        serde_json::Value::Bool(true),
    );
    metadata.insert("ephemeral".to_string(), serde_json::Value::Bool(true));

    messages.push(Message {
        id: Uuid::new_v4(),
        role: MessageRole::System,
        content: MessageContent::Text(text),
        timestamp: Utc::now(),
        metadata,
    });
    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn injects_zh_reminder_without_persist_marker_in_caller_history() {
        let base = vec![Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text("hello".into()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }];
        let ctx = anycode_core::ChatTurnContext {
            dashboard_session_id: None,
            user_turn_id: None,
            reply_language: Some("zh".into()),
            host_intent_hint: None,
        };
        anycode_core::scope_chat_turn(ctx, async {
            let injected = inject_ephemeral_reply_language_reminder(base.clone());
            assert_eq!(injected.len(), base.len() + 1);
            assert!(matches!(
                injected.last().map(|m| &m.role),
                Some(MessageRole::System)
            ));
            assert_eq!(base.len(), 1);
        })
        .await;
    }

    #[tokio::test]
    async fn injects_host_intent_without_reply_language() {
        let base = vec![Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text("启动".into()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }];
        let ctx = anycode_core::ChatTurnContext {
            dashboard_session_id: None,
            user_turn_id: None,
            reply_language: None,
            host_intent_hint: Some("【启动本地站点 — 强制】must Bash".into()),
        };
        anycode_core::scope_chat_turn(ctx, async {
            let injected = inject_ephemeral_reply_language_reminder(base.clone());
            let last = injected.last().expect("reminder");
            match &last.content {
                MessageContent::Text(t) => assert!(t.contains("强制")),
                _ => panic!("expected text"),
            }
        })
        .await;
    }

    #[tokio::test]
    async fn reminder_follows_scoped_reply_language() {
        let ctx = anycode_core::ChatTurnContext {
            dashboard_session_id: None,
            user_turn_id: None,
            reply_language: Some("zh".into()),
            host_intent_hint: None,
        };
        anycode_core::scope_chat_turn(ctx, async {
            let text = ephemeral_reply_language_reminder_text().expect("zh reminder");
            assert!(text.contains("中文"));
        })
        .await;
    }
}
