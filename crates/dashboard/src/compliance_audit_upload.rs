//! Persist-and-forward queue for cloud conversation compliance audit.

use crate::db::DashboardDb;
use anyhow::Result;
use chrono::Utc;
use regex::Regex;
use serde_json::json;
use sqlx::Row;
use std::sync::LazyLock;
use uuid::Uuid;

const MAX_ATTEMPTS: i32 = 8;
const BATCH_LIMIT: i64 = 32;

static CREDENTIAL_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"(?i)(api[_-]?key|secret|token|password)\s*[:=]\s*\S+",
        r"(?i)bearer\s+[a-z0-9._-]{12,}",
        r"sk-[a-zA-Z0-9]{16,}",
    ]
    .into_iter()
    .filter_map(|pattern| Regex::new(pattern).ok())
    .collect()
});

#[derive(Debug, Clone)]
pub struct AuditQueueItem {
    pub id: String,
    pub session_id: String,
    pub conversation_id: String,
    pub message_id: String,
    pub role: String,
    pub content: String,
    pub occurred_at: String,
    pub attempts: i32,
}

pub fn sanitize_audit_content(input: &str) -> String {
    let mut out = input.to_string();
    for pattern in CREDENTIAL_PATTERNS.iter() {
        out = pattern
            .replace_all(&out, "[REDACTED_CREDENTIAL]")
            .to_string();
    }
    out
}

pub async fn enqueue_chat_message(
    db: &DashboardDb,
    session_id: &str,
    role: &str,
    content: &str,
    occurred_at: &str,
) -> Result<()> {
    // Compliance upload is strictly opt-in through a hosted cloud model.
    // Local/offline conversations must never be queued for later upload.
    let model: Option<String> = sqlx::query_scalar("SELECT model FROM sessions WHERE id = ?")
        .bind(session_id)
        .fetch_optional(db.pool())
        .await?
        .flatten();
    if model
        .as_deref()
        .is_some_and(|value| !matches!(value, "auto" | "agnes-chat"))
    {
        return Ok(());
    }
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    if role != "user" && role != "assistant" {
        return Ok(());
    }
    let id = format!("caq_{}", Uuid::new_v4());
    let conversation_id = session_id.to_string();
    let message_id = format!("{session_id}:{role}:{occurred_at}");
    let sanitized = sanitize_audit_content(trimmed);
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO compliance_audit_queue
          (id, session_id, conversation_id, message_id, role, content, occurred_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(session_id)
    .bind(conversation_id)
    .bind(message_id)
    .bind(role)
    .bind(sanitized)
    .bind(occurred_at)
    .execute(db.pool())
    .await?;
    Ok(())
}

pub async fn pending_batch(db: &DashboardDb) -> Result<Vec<AuditQueueItem>> {
    sqlx::query(
        "DELETE FROM compliance_audit_queue WHERE created_at < datetime('now', '-180 days')",
    )
    .execute(db.pool())
    .await?;
    let rows = sqlx::query(
        r#"
        SELECT q.id, q.session_id, q.conversation_id, q.message_id, q.role,
               q.content, q.occurred_at, q.attempts
        FROM compliance_audit_queue q
        JOIN sessions s ON s.id = q.session_id
        WHERE q.attempts < ? AND s.model IN ('auto', 'agnes-chat')
        ORDER BY q.created_at ASC
        LIMIT ?
        "#,
    )
    .bind(MAX_ATTEMPTS)
    .bind(BATCH_LIMIT)
    .fetch_all(db.pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| AuditQueueItem {
            id: row.get("id"),
            session_id: row.get("session_id"),
            conversation_id: row.get("conversation_id"),
            message_id: row.get("message_id"),
            role: row.get("role"),
            content: row.get("content"),
            occurred_at: row.get("occurred_at"),
            attempts: row.get("attempts"),
        })
        .collect())
}

pub async fn mark_uploaded(db: &DashboardDb, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM compliance_audit_queue WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn mark_failed(db: &DashboardDb, id: &str, error: &str) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE compliance_audit_queue
        SET attempts = attempts + 1, last_error = ?
        WHERE id = ?
        "#,
    )
    .bind(error.chars().take(500).collect::<String>())
    .bind(id)
    .execute(db.pool())
    .await?;
    Ok(())
}

pub async fn flush_pending(db: &DashboardDb) -> Result<usize> {
    let token = match anycode_llm::read_cloud_access_token() {
        Some(token) if !token.trim().is_empty() => token,
        _ => return Ok(0),
    };
    let items = pending_batch(db).await?;
    if items.is_empty() {
        return Ok(0);
    }
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/audit/messages",
        anycode_llm::account_api_url().trim_end_matches('/')
    );
    let mut uploaded = 0usize;
    for item in items {
        let occurred_at = chrono::DateTime::parse_from_rfc3339(&item.occurred_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let body = json!({
            "conversation_id": item.conversation_id,
            "message_id": item.message_id,
            "role": item.role,
            "content": item.content,
            "occurred_at": occurred_at,
            "source": "cloud",
        });
        let response = client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await;
        match response {
            Ok(resp) if resp.status().is_success() => {
                mark_uploaded(db, &item.id).await?;
                uploaded += 1;
            }
            Ok(resp) if resp.status() == reqwest::StatusCode::UNAUTHORIZED => {
                if let Ok(new_token) = anycode_llm::refresh_cloud_access_token().await {
                    let retry = client
                        .post(&url)
                        .bearer_auth(&new_token)
                        .json(&body)
                        .send()
                        .await;
                    if matches!(retry, Ok(r) if r.status().is_success()) {
                        mark_uploaded(db, &item.id).await?;
                        uploaded += 1;
                        continue;
                    }
                }
                mark_failed(db, &item.id, "unauthorized").await?;
            }
            Ok(resp) => {
                mark_failed(db, &item.id, &format!("status {}", resp.status())).await?;
            }
            Err(err) => {
                mark_failed(db, &item.id, &err.to_string()).await?;
            }
        }
    }
    Ok(uploaded)
}

#[cfg(test)]
mod tests {
    use super::sanitize_audit_content;

    #[test]
    fn redacts_common_credential_patterns() {
        let input = "api_key=sk-test1234567890 and Bearer abcdefghijklmnop";
        let out = sanitize_audit_content(input);
        assert!(!out.contains("sk-test"));
        assert!(!out.contains("Bearer abc"));
        assert!(out.contains("[REDACTED_CREDENTIAL]"));
    }
}
