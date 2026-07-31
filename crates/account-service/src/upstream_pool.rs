use crate::crypto::{decrypt_secret, encrypt_secret};
use crate::db::AccountDb;
use anyhow::{anyhow, Result};
use chrono::{Datelike, Timelike, Utc};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct UpstreamAccountView {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub status: String,
    pub weight: i32,
    pub concurrency_limit: i32,
    pub rpm_limit: i32,
    pub tpm_limit: i64,
    pub failure_count: i32,
    pub cooldown_until: Option<String>,
    pub tags: Option<String>,
    pub notes: Option<String>,
    pub has_active_key: bool,
}

#[derive(Debug, Clone)]
pub struct UpstreamCredential {
    pub account_id: String,
    pub key_id: String,
    pub api_key: String,
    pub base_url: String,
}

pub fn default_agnes_base_url() -> String {
    std::env::var("AGNES_API_BASE_URL")
        .unwrap_or_else(|_| "https://apihub.agnes-ai.com/v1/chat/completions".into())
}

pub fn default_deepseek_base_url() -> String {
    std::env::var("DEEPSEEK_API_BASE_URL").unwrap_or_else(|_| "https://api.deepseek.com".into())
}

pub fn default_upstream_base_url(provider_id: &str) -> String {
    match provider_id {
        "deepseek" => default_deepseek_base_url(),
        _ => default_agnes_base_url(),
    }
}

pub async fn list_upstream_accounts(
    db: &AccountDb,
    provider_id: Option<&str>,
) -> Result<Vec<UpstreamAccountView>> {
    let rows = if let Some(pid) = provider_id {
        sqlx::query(
            r#"
            SELECT a.id, a.provider_id, a.name, a.status, a.weight, a.concurrency_limit,
                   a.rpm_limit, a.tpm_limit, a.failure_count, a.cooldown_until, a.tags, a.notes,
                   (SELECT COUNT(*) FROM upstream_account_keys k
                    WHERE k.account_id = a.id AND k.status = 'active' AND k.revoked_at IS NULL) AS key_count
            FROM upstream_accounts a
            WHERE a.provider_id = ?
            ORDER BY a.name ASC
            "#,
        )
        .bind(pid)
        .fetch_all(db.pool())
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT a.id, a.provider_id, a.name, a.status, a.weight, a.concurrency_limit,
                   a.rpm_limit, a.tpm_limit, a.failure_count, a.cooldown_until, a.tags, a.notes,
                   (SELECT COUNT(*) FROM upstream_account_keys k
                    WHERE k.account_id = a.id AND k.status = 'active' AND k.revoked_at IS NULL) AS key_count
            FROM upstream_accounts a
            ORDER BY a.provider_id ASC, a.name ASC
            "#,
        )
        .fetch_all(db.pool())
        .await?
    };

    Ok(rows
        .into_iter()
        .map(|r| {
            let cooldown: Option<chrono::DateTime<Utc>> = r.get("cooldown_until");
            let key_count: i64 = r.get("key_count");
            UpstreamAccountView {
                id: r.get("id"),
                provider_id: r.get("provider_id"),
                name: r.get("name"),
                status: r.get("status"),
                weight: r.get("weight"),
                concurrency_limit: r.get("concurrency_limit"),
                rpm_limit: r.get("rpm_limit"),
                tpm_limit: r.get("tpm_limit"),
                failure_count: r.get("failure_count"),
                cooldown_until: cooldown.map(|t| t.to_rfc3339()),
                tags: r.get("tags"),
                notes: r.get("notes"),
                has_active_key: key_count > 0,
            }
        })
        .collect())
}

pub async fn create_upstream_account(
    db: &AccountDb,
    master_secret: &str,
    provider_id: &str,
    name: &str,
    api_key: &str,
    base_url: Option<&str>,
    weight: i32,
) -> Result<UpstreamAccountView> {
    let account_id = format!("uacc_{}", Uuid::new_v4());
    let key_id = format!("ukey_{}", Uuid::new_v4());
    let (ciphertext, nonce) = encrypt_secret(api_key, master_secret)?;
    let url = base_url
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_upstream_base_url(provider_id));

    let mut tx = db.pool().begin().await?;
    sqlx::query(
        r#"
        INSERT INTO upstream_accounts (id, provider_id, name, status, weight)
        VALUES (?, ?, ?, 'active', ?)
        "#,
    )
    .bind(&account_id)
    .bind(provider_id)
    .bind(name)
    .bind(weight)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO upstream_account_keys (id, account_id, key_ciphertext, key_nonce, base_url, status)
        VALUES (?, ?, ?, ?, ?, 'active')
        "#,
    )
    .bind(&key_id)
    .bind(&account_id)
    .bind(&ciphertext)
    .bind(&nonce)
    .bind(&url)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    list_upstream_accounts(db, Some(provider_id))
        .await?
        .into_iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| anyhow!("created account not found"))
}

pub async fn update_upstream_account_status(
    db: &AccountDb,
    account_id: &str,
    status: &str,
) -> Result<()> {
    sqlx::query("UPDATE upstream_accounts SET status = ?, updated_at = NOW() WHERE id = ?")
        .bind(status)
        .bind(account_id)
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn update_upstream_account_weight(
    db: &AccountDb,
    account_id: &str,
    weight: i32,
) -> Result<()> {
    sqlx::query("UPDATE upstream_accounts SET weight = ?, updated_at = NOW() WHERE id = ?")
        .bind(weight)
        .bind(account_id)
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn select_upstream_credential(
    db: &AccountDb,
    master_secret: &str,
    provider_id: &str,
    exclude_account_ids: &[String],
) -> Result<Option<UpstreamCredential>> {
    let rows = sqlx::query(
        r#"
        SELECT a.id AS account_id, a.weight, a.rpm_limit, a.tpm_limit, a.failure_count,
               k.id AS key_id, k.key_ciphertext, k.key_nonce, k.base_url
        FROM upstream_accounts a
        JOIN upstream_account_keys k ON k.account_id = a.id
        WHERE a.provider_id = ?
          AND a.status = 'active'
          AND k.status = 'active'
          AND k.revoked_at IS NULL
          AND (a.cooldown_until IS NULL OR a.cooldown_until <= NOW())
        ORDER BY a.weight DESC, a.failure_count ASC, a.updated_at ASC
        "#,
    )
    .bind(provider_id)
    .fetch_all(db.pool())
    .await?;

    for r in rows {
        let account_id: String = r.get("account_id");
        if exclude_account_ids.iter().any(|id| id == &account_id) {
            continue;
        }
        let rpm_limit: i32 = r.get("rpm_limit");
        let tpm_limit: i64 = r.get("tpm_limit");
        if !within_rate_limits(db, &account_id, rpm_limit, tpm_limit).await? {
            continue;
        }
        let api_key = decrypt_secret(
            &r.get::<String, _>("key_ciphertext"),
            &r.get::<String, _>("key_nonce"),
            master_secret,
        )?;
        let base_url: Option<String> = r.get("base_url");
        let provider = provider_id.to_string();
        return Ok(Some(UpstreamCredential {
            account_id,
            key_id: r.get("key_id"),
            api_key,
            base_url: base_url
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| default_upstream_base_url(&provider)),
        }));
    }
    Ok(None)
}

async fn within_rate_limits(
    db: &AccountDb,
    account_id: &str,
    rpm_limit: i32,
    tpm_limit: i64,
) -> Result<bool> {
    let minute_start = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let row = sqlx::query(
        r#"
        SELECT requests_count, prompt_tokens, completion_tokens
        FROM upstream_account_usage_windows
        WHERE account_id = ? AND window_type = 'minute' AND window_start = ?
        "#,
    )
    .bind(account_id)
    .bind(minute_start)
    .fetch_optional(db.pool())
    .await?;

    if let Some(r) = row {
        let requests: i32 = r.get("requests_count");
        let prompt: i64 = r.get("prompt_tokens");
        let completion: i64 = r.get("completion_tokens");
        if requests >= rpm_limit {
            return Ok(false);
        }
        if prompt + completion >= tpm_limit {
            return Ok(false);
        }
    }
    Ok(true)
}

pub async fn record_upstream_success(
    db: &AccountDb,
    account_id: &str,
    prompt_tokens: i64,
    completion_tokens: i64,
) -> Result<()> {
    let now = Utc::now();
    let minute_start = now.with_second(0).unwrap().with_nanosecond(0).unwrap();
    let day_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
    let month_start = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let mut tx = db.pool().begin().await?;
    for (window_type, window_start) in [
        ("minute", minute_start),
        ("day", day_start.and_utc()),
        ("month", month_start.and_utc()),
    ] {
        upsert_usage_window(
            &mut tx,
            account_id,
            window_type,
            window_start,
            prompt_tokens,
            completion_tokens,
        )
        .await?;
    }

    sqlx::query(
        "UPDATE upstream_accounts SET failure_count = 0, cooldown_until = NULL, updated_at = NOW() WHERE id = ?",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

async fn upsert_usage_window(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    account_id: &str,
    window_type: &str,
    window_start: chrono::DateTime<Utc>,
    prompt_tokens: i64,
    completion_tokens: i64,
) -> Result<()> {
    let existing = sqlx::query(
        r#"
        SELECT id FROM upstream_account_usage_windows
        WHERE account_id = ? AND window_type = ? AND window_start = ?
        "#,
    )
    .bind(account_id)
    .bind(window_type)
    .bind(window_start)
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(row) = existing {
        let id: String = row.get("id");
        sqlx::query(
            r#"
            UPDATE upstream_account_usage_windows
            SET requests_count = requests_count + 1,
                prompt_tokens = prompt_tokens + ?,
                completion_tokens = completion_tokens + ?
            WHERE id = ?
            "#,
        )
        .bind(prompt_tokens)
        .bind(completion_tokens)
        .bind(&id)
        .execute(&mut **tx)
        .await?;
    } else {
        let id = format!("uw_{}", Uuid::new_v4());
        sqlx::query(
            r#"
            INSERT INTO upstream_account_usage_windows
              (id, account_id, window_type, window_start, requests_count, prompt_tokens, completion_tokens)
            VALUES (?, ?, ?, ?, 1, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(account_id)
        .bind(window_type)
        .bind(window_start)
        .bind(prompt_tokens)
        .bind(completion_tokens)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

pub async fn record_upstream_failure(
    db: &AccountDb,
    account_id: &str,
    status_code: Option<i32>,
    message: &str,
) -> Result<()> {
    let event_id = format!("uhe_{}", Uuid::new_v4());
    let mut tx = db.pool().begin().await?;

    sqlx::query(
        r#"
        INSERT INTO upstream_account_health_events (id, account_id, event_type, status_code, message)
        VALUES (?, ?, 'failure', ?, ?)
        "#,
    )
    .bind(&event_id)
    .bind(account_id)
    .bind(status_code)
    .bind(message)
    .execute(&mut *tx)
    .await?;

    let cooldown_secs = if status_code == Some(429) { 120 } else { 60 };
    sqlx::query(
        r#"
        UPDATE upstream_accounts
        SET failure_count = failure_count + 1,
            cooldown_until = DATE_ADD(NOW(), INTERVAL ? SECOND),
            updated_at = NOW()
        WHERE id = ?
        "#,
    )
    .bind(cooldown_secs)
    .bind(account_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn list_health_events(
    db: &AccountDb,
    account_id: Option<&str>,
    limit: i64,
) -> Result<Vec<serde_json::Value>> {
    let rows = if let Some(aid) = account_id {
        sqlx::query(
            r#"
            SELECT id, account_id, event_type, status_code, message, created_at
            FROM upstream_account_health_events
            WHERE account_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(aid)
        .bind(limit)
        .fetch_all(db.pool())
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT id, account_id, event_type, status_code, message, created_at
            FROM upstream_account_health_events
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(db.pool())
        .await?
    };

    Ok(rows
        .into_iter()
        .map(|r| {
            let created: chrono::DateTime<Utc> = r.get("created_at");
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "account_id": r.get::<String, _>("account_id"),
                "event_type": r.get::<String, _>("event_type"),
                "status_code": r.get::<Option<i32>, _>("status_code"),
                "message": r.get::<Option<String>, _>("message"),
                "created_at": created.to_rfc3339(),
            })
        })
        .collect())
}
