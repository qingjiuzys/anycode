use super::*;
use crate::control::text_upload::TextFilePayload;
use crate::control::vision_payload::VisionImagePayload;
use crate::schema::QueuedMessageRecord;
use serde_json::json;

#[derive(Debug, Clone)]
pub struct EnqueueMessageInput {
    pub session_id: String,
    pub prompt: String,
    pub agent: Option<String>,
    pub skills: Option<Vec<String>>,
    pub vision_images: Option<Vec<VisionImagePayload>>,
    pub text_files: Option<Vec<TextFilePayload>>,
    pub lang: Option<String>,
    pub composer_mode: Option<String>,
}

impl DashboardDb {
    pub async fn enqueue_session_message(
        &self,
        input: EnqueueMessageInput,
    ) -> Result<(QueuedMessageRecord, i64)> {
        let seq: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(MAX(seq), 0) + 1 FROM session_message_queue WHERE session_id = ?
            "#,
        )
        .bind(&input.session_id)
        .fetch_one(&self.pool)
        .await?;

        let id = format!("mq_{}", Uuid::new_v4().simple());
        let skills_json = input
            .skills
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let vision_json = input
            .vision_images
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let text_files_json = input
            .text_files
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        sqlx::query(
            r#"
            INSERT INTO session_message_queue
                (id, session_id, seq, prompt, agent, skills_json, vision_json, text_files_json, lang, composer_mode, status)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending')
            "#,
        )
        .bind(&id)
        .bind(&input.session_id)
        .bind(seq)
        .bind(&input.prompt)
        .bind(&input.agent)
        .bind(&skills_json)
        .bind(&vision_json)
        .bind(&text_files_json)
        .bind(&input.lang)
        .bind(&input.composer_mode)
        .execute(&self.pool)
        .await?;

        let pending_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM session_message_queue
            WHERE session_id = ? AND status = 'pending'
            "#,
        )
        .bind(&input.session_id)
        .fetch_one(&self.pool)
        .await?;

        Ok((
            QueuedMessageRecord {
                id,
                session_id: input.session_id,
                seq,
                prompt: input.prompt,
                agent: input.agent,
                status: "pending".into(),
                created_at: chrono::Utc::now().to_rfc3339(),
                error: None,
            },
            pending_count,
        ))
    }

    pub async fn list_pending_session_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<QueuedMessageRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT id, session_id, seq, prompt, agent, status, created_at, error
            FROM session_message_queue
            WHERE session_id = ? AND status = 'pending'
            ORDER BY seq ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| QueuedMessageRecord {
                id: r.get("id"),
                session_id: r.get("session_id"),
                seq: r.get("seq"),
                prompt: r.get("prompt"),
                agent: r.get("agent"),
                status: r.get("status"),
                created_at: r.get("created_at"),
                error: r.get("error"),
            })
            .collect())
    }

    pub async fn cancel_queued_message(&self, session_id: &str, queue_id: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE session_message_queue
            SET status = 'cancelled'
            WHERE id = ? AND session_id = ? AND status = 'pending'
            "#,
        )
        .bind(queue_id)
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn cancel_all_pending_queue_messages(&self, session_id: &str) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE session_message_queue
            SET status = 'cancelled'
            WHERE session_id = ? AND status = 'pending'
            "#,
        )
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn pop_next_pending_queue_message(
        &self,
        session_id: &str,
    ) -> Result<Option<QueuedMessagePop>> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            r#"
            SELECT id, session_id, seq, prompt, agent, skills_json, vision_json, text_files_json, lang, composer_mode
            FROM session_message_queue
            WHERE session_id = ? AND status = 'pending'
            ORDER BY seq ASC
            LIMIT 1
            "#,
        )
        .bind(session_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.commit().await?;
            return Ok(None);
        };

        let id: String = row.get("id");
        let updated = sqlx::query(
            r#"
            UPDATE session_message_queue
            SET status = 'sent', sent_at = datetime('now')
            WHERE id = ? AND status = 'pending'
            "#,
        )
        .bind(&id)
        .execute(&mut *tx)
        .await?;
        if updated.rows_affected() == 0 {
            tx.commit().await?;
            return Ok(None);
        }
        tx.commit().await?;

        let skills_json: Option<String> = row.get("skills_json");
        let vision_json: Option<String> = row.get("vision_json");
        let text_files_json: Option<String> = row.get("text_files_json");

        Ok(Some(QueuedMessagePop {
            id,
            session_id: row.get("session_id"),
            seq: row.get("seq"),
            prompt: row.get("prompt"),
            agent: row.get("agent"),
            skills: skills_json
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
            vision_images: vision_json
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
            text_files: text_files_json
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
            lang: row.get("lang"),
            composer_mode: row.get("composer_mode"),
        }))
    }

    pub async fn mark_queue_message_failed(&self, queue_id: &str, error: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE session_message_queue
            SET status = 'failed', error = ?
            WHERE id = ?
            "#,
        )
        .bind(error)
        .bind(queue_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_message_queued_event(
        &self,
        project_id: &str,
        session_id: &str,
        queue_id: &str,
        position: i64,
        prompt: &str,
    ) -> Result<crate::schema::ProjectEvent> {
        self.insert_event(crate::schema::InsertEventRequest {
            project_id: project_id.to_string(),
            session_id: Some(session_id.to_string()),
            task_id: None,
            agent_id: None,
            event_type: "message_queued".into(),
            severity: Some("info".into()),
            title: "Message queued".into(),
            body: Some(prompt.chars().take(8000).collect()),
            payload: Some(json!({
                "queue_id": queue_id,
                "position": position,
                "source": "message_queue",
                "status": "pending",
            })),
        })
        .await
    }
}

#[derive(Debug, Clone)]
pub struct QueuedMessagePop {
    pub id: String,
    pub session_id: String,
    pub seq: i64,
    pub prompt: String,
    pub agent: Option<String>,
    pub skills: Option<Vec<String>>,
    pub vision_images: Option<Vec<VisionImagePayload>>,
    pub text_files: Option<Vec<TextFilePayload>>,
    pub lang: Option<String>,
    pub composer_mode: Option<String>,
}
