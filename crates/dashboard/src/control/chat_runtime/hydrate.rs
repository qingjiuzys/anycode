//! Rebuild LLM message history from persisted `chat_turn_events`.
//!
//! Embedded chat keeps an in-memory `Vec<Message>` for follow-ups. After process
//! restart or session eviction that map is empty while the UI still shows the
//! transcript from SQLite — hydrate user + final assistant text so the next
//! turn can see prior answers.

use crate::db::DashboardDb;
use crate::observability::chat_turn_log::records_to_transcript_blocks;
use crate::schema::ChatTurnEventRecord;
use anycode_core::prelude::*;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

/// Cap for a single hydrate load (matches session SSE replay budget).
const HYDRATE_EVENT_LIMIT: i64 = 10_000;

#[derive(Debug, Default, Clone)]
pub struct HydratedHistory {
    pub messages: Vec<Message>,
    /// Highest `conversation_turn_id` seen; next send uses `max + 1`.
    pub max_user_turn_id: u32,
}

/// Load prior user/assistant turns for `session_id` from the canonical event log.
pub async fn load_prior_history(db: &DashboardDb, session_id: &str) -> Result<HydratedHistory> {
    let records = db
        .list_recent_chat_turn_events(session_id, HYDRATE_EVENT_LIMIT)
        .await?;
    // The next user turn id must come from the global max, not the truncated
    // window — otherwise long sessions collide with existing turn ids.
    let max_user_turn_id = db.max_conversation_turn_id(session_id).await.unwrap_or(0) as u32;
    let mut history = history_from_records(&records);
    history.max_user_turn_id = max_user_turn_id;
    Ok(history)
}

/// Map persisted events → LLM messages (user + assistant bodies only).
#[must_use]
pub fn history_from_records(records: &[ChatTurnEventRecord]) -> HydratedHistory {
    let max_user_turn_id = records
        .iter()
        .map(|r| r.conversation_turn_id)
        .max()
        .unwrap_or(0);
    let blocks = records_to_transcript_blocks(records);
    let mut messages = Vec::new();
    for block in blocks {
        let role = match block.block_type.as_str() {
            "user_message" => MessageRole::User,
            "assistant_message" => MessageRole::Assistant,
            _ => continue,
        };
        // User bubbles store display text in body; OCR/model appendix lives in meta.model_prompt.
        let content_text = if role == MessageRole::User {
            block
                .meta
                .get("model_prompt")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(block.body.trim())
                .to_string()
        } else {
            block.body.trim().to_string()
        };
        if content_text.is_empty() {
            continue;
        }
        messages.push(Message {
            id: Uuid::new_v4(),
            role,
            content: MessageContent::Text(content_text),
            timestamp: parse_occurred_at(&block.at),
            metadata: HashMap::new(),
        });
    }
    HydratedHistory {
        messages,
        max_user_turn_id,
    }
}

fn parse_occurred_at(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DashboardDb;
    use crate::observability::chat_events::assistant_delta_event;
    use crate::observability::chat_turn_log::{persist_and_enrich, user_message_event};
    use crate::schema::{CreateSessionRequest, UpsertProjectRequest};

    fn plain_text(msg: &Message) -> &str {
        match &msg.content {
            MessageContent::Text(t) => t.as_str(),
            _ => "",
        }
    }

    #[test]
    fn history_from_records_prefers_model_prompt_for_user() {
        let records = vec![ChatTurnEventRecord {
            id: "1".into(),
            session_id: "s".into(),
            project_id: "p".into(),
            conversation_turn_id: 1,
            agent_turn: None,
            seq: 1,
            kind: "user_message".into(),
            tool_key: None,
            tool_name: None,
            body: "这是什么".into(),
            block_json: Some(
                r#"{"id":"u1","block_type":"user_message","at":"2026-01-01T00:00:00Z","title":"User","body":"这是什么","meta":{"model_prompt":"这是什么\n\n--- OCR ---\nhi"},"collapsible":false,"default_collapsed":false}"#.into(),
            ),
            payload: serde_json::json!({}),
            occurred_at: "2026-01-01T00:00:00Z".into(),
        }];
        let history = history_from_records(&records);
        assert_eq!(history.messages.len(), 1);
        assert!(plain_text(&history.messages[0]).contains("OCR"));
        assert!(plain_text(&history.messages[0]).contains("这是什么"));
    }

    #[test]
    fn history_from_records_keeps_user_and_final_assistant_order() {
        let records = vec![
            ChatTurnEventRecord {
                id: "1".into(),
                session_id: "s".into(),
                project_id: "p".into(),
                conversation_turn_id: 1,
                agent_turn: None,
                seq: 1,
                kind: "user_message".into(),
                tool_key: None,
                tool_name: None,
                body: "分析项目".into(),
                block_json: Some(
                    r#"{"id":"u1","block_type":"user_message","at":"2026-01-01T00:00:00Z","title":"User","body":"分析项目","meta":{},"collapsible":false,"default_collapsed":false}"#.into(),
                ),
                payload: serde_json::json!({}),
                occurred_at: "2026-01-01T00:00:00Z".into(),
            },
            ChatTurnEventRecord {
                id: "2".into(),
                session_id: "s".into(),
                project_id: "p".into(),
                conversation_turn_id: 1,
                agent_turn: Some(1),
                seq: 2,
                kind: "assistant_delta".into(),
                tool_key: None,
                tool_name: None,
                body: "由 zeenyun 团队开发".into(),
                block_json: Some(
                    r#"{"id":"a1","block_type":"assistant_message","at":"2026-01-01T00:01:00Z","title":"Assistant","body":"由 zeenyun 团队开发","meta":{"live":true},"collapsible":false,"default_collapsed":false}"#.into(),
                ),
                payload: serde_json::json!({}),
                occurred_at: "2026-01-01T00:01:00Z".into(),
            },
        ];
        // Rebuild via record_to_stream_event path — block_json drives body.
        let history = history_from_records(&records);
        assert_eq!(history.max_user_turn_id, 1);
        assert_eq!(history.messages.len(), 2);
        assert_eq!(history.messages[0].role, MessageRole::User);
        assert_eq!(plain_text(&history.messages[0]), "分析项目");
        assert_eq!(history.messages[1].role, MessageRole::Assistant);
        assert!(plain_text(&history.messages[1]).contains("zeenyun"));
    }

    #[tokio::test]
    async fn load_prior_history_from_db_after_persist() {
        let dir = tempfile::tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("hydrate.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/hydrate".into(),
                name: Some("Hydrate".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "repl".into(),
                task_id: None,
                title: "Hydrate".into(),
                prompt_preview: Some("hello".into()),
                agent_type: Some("general-purpose".into()),
                model: Some("test".into()),
                metadata_json: None,
            })
            .await
            .unwrap();

        let user_evt = user_message_event(&session.id, &project.id, 1, "分析下当前应用");
        persist_and_enrich(&db, user_evt, 1).await.unwrap();
        let assistant_evt = assistant_delta_event(
            &session.id,
            &project.id,
            1,
            1,
            "由 zeenyun 团队开发的灵栖 BMS",
            "由 zeenyun 团队开发的灵栖 BMS",
            false,
        );
        persist_and_enrich(&db, assistant_evt, 1).await.unwrap();

        let history = load_prior_history(&db, &session.id).await.unwrap();
        assert_eq!(history.max_user_turn_id, 1);
        assert_eq!(history.messages.len(), 2);
        assert_eq!(plain_text(&history.messages[0]), "分析下当前应用");
        assert!(plain_text(&history.messages[1]).contains("zeenyun"));
    }
}
