//! Backfill `llm_usage` index events from historical `output.log` files.

use crate::db::DashboardDb;
use crate::log_parser::parse_line;
use crate::observability::llm_usage::{self, EVENT_TYPE as LLM_USAGE_EVENT};
use crate::schema::InsertEventRequest;
use anyhow::Result;
use std::path::Path;

const MAX_SESSIONS: usize = 500;

/// Scan sessions with task logs and insert missing `llm_usage` rows (idempotent).
pub async fn backfill_llm_usage(db: &DashboardDb, tasks_root: &Path) -> Result<usize> {
    use sqlx::Row;
    let rows = sqlx::query(
        r#"
        SELECT id, project_id, task_id
        FROM sessions
        WHERE task_id IS NOT NULL AND TRIM(task_id) != ''
        ORDER BY datetime(started_at) DESC
        LIMIT ?
        "#,
    )
    .bind(MAX_SESSIONS as i64)
    .fetch_all(db.pool())
    .await?;

    let mut inserted = 0usize;
    for row in rows {
        let session_id: String = row.get("id");
        let project_id: String = row.get("project_id");
        let task_id: String = row.get("task_id");
        let log_path = tasks_root.join(&task_id).join("output.log");
        if !log_path.is_file() {
            continue;
        }
        let content = match std::fs::read_to_string(&log_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for line in content.lines() {
            let Some(parsed) = parse_line(line) else {
                continue;
            };
            if parsed.event_type != "llm_response_end" {
                continue;
            }
            let Some(payload) = llm_usage::usage_payload_from_parsed(&parsed) else {
                continue;
            };
            let turn = payload.get("turn").and_then(|v| v.as_str()).unwrap_or("0");
            let exists: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM project_events
                WHERE session_id = ?
                  AND event_type = ?
                  AND json_extract(payload_json, '$.turn') = ?
                "#,
            )
            .bind(&session_id)
            .bind(LLM_USAGE_EVENT)
            .bind(turn)
            .fetch_one(db.pool())
            .await?;
            if exists > 0 {
                continue;
            }
            let input = payload
                .get("input_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let output = payload
                .get("output_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let title = format!("LLM usage ({input} in / {output} out tokens)");
            let _ = db
                .insert_event(InsertEventRequest {
                    project_id: project_id.clone(),
                    session_id: Some(session_id.clone()),
                    task_id: Some(task_id.clone()),
                    agent_id: None,
                    event_type: LLM_USAGE_EVENT.into(),
                    severity: Some("info".into()),
                    title,
                    body: None,
                    payload: Some(payload),
                })
                .await?;
            inserted += 1;
        }
    }
    Ok(inserted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{CreateSessionRequest, UpsertProjectRequest};
    use tempfile::tempdir;

    #[tokio::test]
    async fn backfill_inserts_llm_usage_from_log() {
        let dir = tempdir().unwrap();
        let tasks_root = dir.path().join("tasks");
        let task_id = "task-backfill-1";
        let log_dir = tasks_root.join(task_id);
        std::fs::create_dir_all(&log_dir).unwrap();
        std::fs::write(
            log_dir.join("output.log"),
            "[llm_response_end] turn=1 elapsed_ms=100 input_tokens=42 output_tokens=7\n",
        )
        .unwrap();

        let db = DashboardDb::open(dir.path().join("bf.db")).await.unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/backfill".into(),
                name: Some("BF".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        db.create_session(CreateSessionRequest {
            project_id: project.id,
            kind: "run".into(),
            task_id: Some(task_id.into()),
            title: "backfill".into(),
            prompt_preview: None,
            agent_type: None,
            model: Some("claude-sonnet-4".into()),
            metadata_json: None,
        })
        .await
        .unwrap();

        let n = backfill_llm_usage(&db, &tasks_root).await.unwrap();
        assert_eq!(n, 1);

        let usage = crate::metrics::global_token_usage_detail(&db, 7)
            .await
            .unwrap();
        assert_eq!(usage.usage.llm_calls, 1);
        assert_eq!(usage.usage.input_tokens, 42);
        assert_eq!(usage.usage.output_tokens, 7);

        let again = backfill_llm_usage(&db, &tasks_root).await.unwrap();
        assert_eq!(again, 0);
    }
}
