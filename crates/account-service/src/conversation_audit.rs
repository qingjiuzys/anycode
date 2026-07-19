use crate::{auth::hash_token, crypto, db::AccountDb};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::{Duration, Utc};
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

pub const RETENTION_DAYS: i64 = 180;

#[derive(Debug, Deserialize)]
pub struct AuditIngest {
    pub conversation_id: String,
    pub message_id: String,
    pub role: String,
    pub content: String,
    pub occurred_at: Option<chrono::DateTime<Utc>>,
    #[serde(default = "cloud_source")]
    pub source: String,
}

fn cloud_source() -> String {
    "cloud".into()
}

pub async fn ingest(
    db: &AccountDb,
    organization_id: &str,
    user_id: &str,
    body: &AuditIngest,
    master_secret: &str,
) -> Result<usize> {
    if body.source != "cloud" {
        return Err(anyhow!(
            "only cloud conversations may be uploaded for audit"
        ));
    }
    if body.content.len() > 1_000_000 || body.conversation_id.trim().is_empty() {
        return Err(anyhow!("invalid audit message"));
    }
    let occurred = body.occurred_at.unwrap_or_else(Utc::now);
    let expires = occurred + Duration::days(RETENTION_DAYS);
    let conversation_db_id = format!("aconv_{}", Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO audit_conversations
          (id, organization_id, user_id, client_conversation_id, source, started_at, last_message_at, expires_at)
        VALUES (?, ?, ?, ?, 'cloud', ?, ?, ?)
        ON DUPLICATE KEY UPDATE last_message_at = GREATEST(last_message_at, VALUES(last_message_at)),
          expires_at = GREATEST(expires_at, VALUES(expires_at))
        "#,
    )
    .bind(&conversation_db_id)
    .bind(organization_id)
    .bind(user_id)
    .bind(&body.conversation_id)
    .bind(occurred)
    .bind(occurred)
    .bind(expires)
    .execute(db.pool())
    .await?;
    let actual_conversation_id: String = sqlx::query_scalar(
        "SELECT id FROM audit_conversations WHERE organization_id = ? AND client_conversation_id = ?",
    )
    .bind(organization_id)
    .bind(&body.conversation_id)
    .fetch_one(db.pool())
    .await?;

    let data_key = B64.encode(rand::random::<[u8; 32]>());
    let (content_ct, content_nonce) = crypto::encrypt_secret(&body.content, &data_key)?;
    let (encrypted_key, key_nonce) = crypto::encrypt_secret(&data_key, master_secret)?;
    let message_db_id = format!("amsg_{}", Uuid::new_v4());
    let inserted = sqlx::query(
        r#"
        INSERT IGNORE INTO audit_messages
          (id, conversation_id, client_message_id, role, content_ciphertext, content_nonce,
           encrypted_data_key, data_key_nonce, content_sha256, occurred_at, expires_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&message_db_id)
    .bind(&actual_conversation_id)
    .bind(&body.message_id)
    .bind(&body.role)
    .bind(content_ct)
    .bind(content_nonce)
    .bind(encrypted_key)
    .bind(key_nonce)
    .bind(hash_token(&body.content))
    .bind(occurred)
    .bind(expires)
    .execute(db.pool())
    .await?
    .rows_affected();
    if inserted == 0 {
        return Ok(0);
    }

    let rules =
        sqlx::query("SELECT id, keyword, severity FROM audit_keyword_rules WHERE enabled = 1")
            .fetch_all(db.pool())
            .await?;
    let normalized = body.content.to_lowercase();
    let mut hits = 0;
    for rule in rules {
        let keyword: String = rule.get("keyword");
        if !keyword.is_empty() && normalized.contains(&keyword.to_lowercase()) {
            sqlx::query(
                "INSERT IGNORE INTO audit_keyword_hits (id, message_id, rule_id, severity, matched_excerpt_masked) VALUES (?, ?, ?, ?, '[MATCH REDACTED]')",
            )
            .bind(format!("ahit_{}", Uuid::new_v4()))
            .bind(&message_db_id)
            .bind(rule.get::<String, _>("id"))
            .bind(rule.get::<String, _>("severity"))
            .execute(db.pool())
            .await?;
            hits += 1;
        }
    }
    Ok(hits)
}

/// Deletes expired audit data. Intended for a daily scheduler/cron invocation.
pub async fn purge_expired(db: &AccountDb) -> Result<u64> {
    let mut tx = db.pool().begin().await?;
    sqlx::query("DELETE FROM audit_messages WHERE expires_at < NOW()")
        .execute(&mut *tx)
        .await?;
    let deleted = sqlx::query("DELETE FROM audit_conversations WHERE expires_at < NOW()")
        .execute(&mut *tx)
        .await?
        .rows_affected();
    sqlx::query(
        "DELETE FROM email_verification_codes WHERE expires_at < DATE_SUB(NOW(), INTERVAL 1 DAY)",
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(deleted)
}
