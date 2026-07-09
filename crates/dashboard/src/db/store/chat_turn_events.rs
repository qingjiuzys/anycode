use crate::schema::ChatTurnEventRecord;
use anyhow::{Context, Result};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use super::DashboardDb;

impl DashboardDb {
    pub async fn next_chat_turn_seq(&self, session_id: &str) -> Result<i64> {
        let next: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM chat_turn_events WHERE session_id = ?",
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(next)
    }

    pub async fn append_chat_turn_event(
        &self,
        session_id: &str,
        project_id: &str,
        conversation_turn_id: u32,
        agent_turn: Option<u32>,
        kind: &str,
        tool_key: Option<&str>,
        tool_name: Option<&str>,
        body: &str,
        block_json: Option<&str>,
        payload: &Value,
        occurred_at: &str,
    ) -> Result<ChatTurnEventRecord> {
        let seq = self.next_chat_turn_seq(session_id).await?;
        let id = format!("cte_{}", Uuid::new_v4().simple());
        let payload_json = serde_json::to_string(payload)?;
        sqlx::query(
            r#"
            INSERT INTO chat_turn_events
              (id, session_id, project_id, conversation_turn_id, agent_turn, seq, kind,
               tool_key, tool_name, body, block_json, payload_json, occurred_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(session_id)
        .bind(project_id)
        .bind(conversation_turn_id as i64)
        .bind(agent_turn.map(|v| v as i64))
        .bind(seq)
        .bind(kind)
        .bind(tool_key)
        .bind(tool_name)
        .bind(body)
        .bind(block_json)
        .bind(&payload_json)
        .bind(occurred_at)
        .execute(&self.pool)
        .await?;
        self.get_chat_turn_event(&id)
            .await?
            .context("chat_turn_event missing after insert")
    }

    pub async fn get_chat_turn_event(&self, id: &str) -> Result<Option<ChatTurnEventRecord>> {
        let row = sqlx::query(
            r#"
            SELECT id, session_id, project_id, conversation_turn_id, agent_turn, seq, kind,
                   tool_key, tool_name, body, block_json, payload_json, occurred_at
            FROM chat_turn_events WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_chat_turn_event))
    }

    pub async fn list_chat_turn_events(
        &self,
        session_id: &str,
        after_seq: Option<i64>,
        limit: i64,
    ) -> Result<Vec<ChatTurnEventRecord>> {
        let rows = if let Some(after) = after_seq {
            sqlx::query(
                r#"
                SELECT id, session_id, project_id, conversation_turn_id, agent_turn, seq, kind,
                       tool_key, tool_name, body, block_json, payload_json, occurred_at
                FROM chat_turn_events
                WHERE session_id = ? AND seq > ?
                ORDER BY seq ASC
                LIMIT ?
                "#,
            )
            .bind(session_id)
            .bind(after)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                r#"
                SELECT id, session_id, project_id, conversation_turn_id, agent_turn, seq, kind,
                       tool_key, tool_name, body, block_json, payload_json, occurred_at
                FROM chat_turn_events
                WHERE session_id = ?
                ORDER BY seq ASC
                LIMIT ?
                "#,
            )
            .bind(session_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(rows.into_iter().map(row_to_chat_turn_event).collect())
    }

    pub async fn chat_turn_event_count(&self, session_id: &str) -> Result<i64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM chat_turn_events WHERE session_id = ?")
                .bind(session_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(count)
    }

    pub async fn max_chat_turn_seq(&self, session_id: &str) -> Result<i64> {
        let max: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) FROM chat_turn_events WHERE session_id = ?",
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(max)
    }
}

fn row_to_chat_turn_event(r: sqlx::sqlite::SqliteRow) -> ChatTurnEventRecord {
    let payload_json: String = r.get("payload_json");
    let payload = serde_json::from_str(&payload_json).unwrap_or(Value::Null);
    ChatTurnEventRecord {
        id: r.get("id"),
        session_id: r.get("session_id"),
        project_id: r.get("project_id"),
        conversation_turn_id: r.get::<i64, _>("conversation_turn_id") as u32,
        agent_turn: r.get::<Option<i64>, _>("agent_turn").map(|v| v as u32),
        seq: r.get("seq"),
        kind: r.get("kind"),
        tool_key: r.get("tool_key"),
        tool_name: r.get("tool_name"),
        body: r.get("body"),
        block_json: r.get("block_json"),
        payload,
        occurred_at: r.get("occurred_at"),
    }
}
