//! Local API token management (SHA-256 hashed at rest).

use crate::audit::{record_audit, AuditEventInput};
use crate::db::DashboardDb;
use crate::schema::ApiTokenRecord;
use anyhow::Result;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

pub struct CreatedToken {
    pub record: ApiTokenRecord,
    pub plaintext: String,
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn generate_token() -> (String, String) {
    let plain = format!("ac_{}", Uuid::new_v4().simple());
    let prefix: String = plain.chars().take(12).collect();
    (plain, prefix)
}

pub async fn create_token(
    db: &DashboardDb,
    name: &str,
    expires_days: Option<i64>,
) -> Result<CreatedToken> {
    let (plaintext, prefix) = generate_token();
    let id = format!("tok_{}", Uuid::new_v4());
    let hash = hash_token(&plaintext);
    let expires_at =
        expires_days.map(|d| (chrono::Utc::now() + chrono::Duration::days(d)).to_rfc3339());
    sqlx::query(
        r#"
        INSERT INTO api_tokens (id, name, token_hash, prefix, expires_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(name)
    .bind(&hash)
    .bind(&prefix)
    .bind(&expires_at)
    .execute(db.pool())
    .await?;
    record_audit(
        db,
        AuditEventInput::low("api_token_created", json!({ "token_id": id, "name": name })),
    )
    .await?;
    Ok(CreatedToken {
        record: ApiTokenRecord {
            id,
            name: name.into(),
            prefix,
            created_at: chrono::Utc::now().to_rfc3339(),
            expires_at,
            last_used_at: None,
            revoked: false,
        },
        plaintext,
    })
}

pub async fn list_tokens(db: &DashboardDb) -> Result<Vec<ApiTokenRecord>> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, prefix, created_at, expires_at, last_used_at, revoked_at
        FROM api_tokens ORDER BY created_at DESC
        "#,
    )
    .fetch_all(db.pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| ApiTokenRecord {
            id: r.get("id"),
            name: r.get("name"),
            prefix: r.get("prefix"),
            created_at: r.get("created_at"),
            expires_at: r.get("expires_at"),
            last_used_at: r.get("last_used_at"),
            revoked: r.get::<Option<String>, _>("revoked_at").is_some(),
        })
        .collect())
}

pub async fn revoke_token(db: &DashboardDb, token_id: &str) -> Result<bool> {
    let res = sqlx::query(
        "UPDATE api_tokens SET revoked_at = datetime('now') WHERE id = ? AND revoked_at IS NULL",
    )
    .bind(token_id)
    .execute(db.pool())
    .await?;
    if res.rows_affected() > 0 {
        record_audit(
            db,
            AuditEventInput::low("api_token_revoked", json!({ "token_id": token_id })),
        )
        .await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn validate_token(db: &DashboardDb, bearer: &str) -> Result<bool> {
    let token = bearer.strip_prefix("Bearer ").unwrap_or(bearer).trim();
    if token.is_empty() {
        return Ok(false);
    }
    let hash = hash_token(token);
    let row = sqlx::query(
        r#"
        SELECT id FROM api_tokens
        WHERE token_hash = ? AND revoked_at IS NULL
          AND (expires_at IS NULL OR datetime(expires_at) > datetime('now'))
        "#,
    )
    .bind(&hash)
    .fetch_optional(db.pool())
    .await?;
    if let Some(r) = row {
        let id: String = r.get("id");
        let _ = sqlx::query("UPDATE api_tokens SET last_used_at = datetime('now') WHERE id = ?")
            .bind(&id)
            .execute(db.pool())
            .await;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn token_count_active(db: &DashboardDb) -> Result<i64> {
    let n: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM api_tokens
        WHERE revoked_at IS NULL
          AND (expires_at IS NULL OR datetime(expires_at) > datetime('now'))
        "#,
    )
    .fetch_one(db.pool())
    .await?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn token_create_validate_revoke() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("t.db")).await.unwrap();
        let created = create_token(&db, "test", None).await.unwrap();
        assert!(validate_token(&db, &created.plaintext).await.unwrap());
        assert!(revoke_token(&db, &created.record.id).await.unwrap());
        assert!(!validate_token(&db, &created.plaintext).await.unwrap());
    }
}
