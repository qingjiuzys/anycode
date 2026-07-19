//! Rolling call-count quota per time window (e.g. 1000 calls every 5 hours).

use crate::db::AccountDb;
use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use sqlx::Row;

pub(crate) const DEFAULT_WINDOW_SECS: i32 = 5 * 3600;

#[derive(Debug, Clone)]
pub struct CallQuotaState {
    pub window_secs: i32,
    pub limit: i32,
    pub used: i32,
    pub window_started_at: chrono::DateTime<Utc>,
    pub resets_at: chrono::DateTime<Utc>,
    pub remaining: i32,
}

pub async fn ensure_window_current(db: &AccountDb, org_id: &str) -> Result<()> {
    let row = sqlx::query(
        r#"
        SELECT calls_limit_per_window, quota_window_secs, quota_window_started_at
        FROM entitlements WHERE organization_id = ?
        "#,
    )
    .bind(org_id)
    .fetch_one(db.pool())
    .await?;

    let limit: i32 = row.get("calls_limit_per_window");
    if limit <= 0 {
        return Ok(());
    }

    let window_secs: i32 = row.get::<i32, _>("quota_window_secs").max(1);
    let now = Utc::now();
    let started: Option<chrono::DateTime<Utc>> = row.get("quota_window_started_at");
    let Some(started) = started else {
        sqlx::query(
            "UPDATE entitlements SET quota_window_started_at = ?, updated_at = NOW() WHERE organization_id = ?",
        )
        .bind(now)
        .bind(org_id)
        .execute(db.pool())
        .await?;
        return Ok(());
    };

    if (now - started).num_seconds() >= i64::from(window_secs) {
        sqlx::query(
            r#"
            UPDATE entitlements SET calls_used_in_window = 0, quota_window_started_at = ?, updated_at = NOW()
            WHERE organization_id = ?
            "#,
        )
        .bind(now)
        .bind(org_id)
        .execute(db.pool())
        .await?;
    }
    Ok(())
}

pub async fn get_call_quota_state(db: &AccountDb, org_id: &str) -> Result<Option<CallQuotaState>> {
    ensure_window_current(db, org_id).await?;
    let row = sqlx::query(
        r#"
        SELECT quota_window_secs, calls_limit_per_window, calls_used_in_window, quota_window_started_at
        FROM entitlements WHERE organization_id = ?
        "#,
    )
    .bind(org_id)
    .fetch_optional(db.pool())
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let limit: i32 = row.get("calls_limit_per_window");
    if limit <= 0 {
        return Ok(None);
    }
    let window_secs: i32 = row.get::<i32, _>("quota_window_secs").max(1);
    let used: i32 = row.get("calls_used_in_window");
    let started: chrono::DateTime<Utc> = row
        .get::<Option<chrono::DateTime<Utc>>, _>("quota_window_started_at")
        .unwrap_or_else(Utc::now);
    let resets_at = started + Duration::seconds(i64::from(window_secs));
    Ok(Some(CallQuotaState {
        window_secs,
        limit,
        used,
        window_started_at: started,
        resets_at,
        remaining: (limit - used).max(0),
    }))
}

pub async fn check_call_quota(db: &AccountDb, org_id: &str) -> Result<()> {
    let Some(state) = get_call_quota_state(db, org_id).await? else {
        return Ok(());
    };
    if state.used >= state.limit {
        return Err(anyhow!(
            "model call quota exceeded ({}/{} calls per {}h; resets at {})",
            state.used,
            state.limit,
            state.window_secs / 3600,
            state.resets_at.to_rfc3339()
        ));
    }
    Ok(())
}

pub async fn record_model_call(db: &AccountDb, org_id: &str) -> Result<()> {
    ensure_window_current(db, org_id).await?;
    let limit: i32 = sqlx::query_scalar(
        "SELECT calls_limit_per_window FROM entitlements WHERE organization_id = ?",
    )
    .bind(org_id)
    .fetch_one(db.pool())
    .await?;
    if limit <= 0 {
        return Ok(());
    }
    let result = sqlx::query(
        r#"
        UPDATE entitlements SET calls_used_in_window = calls_used_in_window + 1, updated_at = NOW()
        WHERE organization_id = ? AND calls_used_in_window < calls_limit_per_window
        "#,
    )
    .bind(org_id)
    .execute(db.pool())
    .await?;
    if result.rows_affected() == 0 {
        return Err(anyhow!("model call quota exceeded"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_WINDOW_SECS;

    #[test]
    fn default_window_is_five_hours() {
        assert_eq!(DEFAULT_WINDOW_SECS, 18_000);
    }
}
