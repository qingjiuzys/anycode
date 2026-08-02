use crate::db::AccountDb;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct PlanLimits {
    pub token_limit: i64,
    pub api_key_limit: i32,
    pub seat_limit: i32,
    pub monthly_price_fen: i32,
    pub yearly_price_fen: i32,
    pub currency: &'static str,
    pub hosted_models_enabled: bool,
    /// Rolling window length in seconds (0 = no call quota).
    pub quota_window_secs: i32,
    /// Max model invocations per window (0 = unlimited by count).
    pub calls_per_window: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudPlanRow {
    pub id: String,
    pub display_name: String,
    pub description: Option<String>,
    pub monthly_price_fen: i32,
    pub yearly_price_fen: i32,
    pub token_limit: i64,
    pub api_key_limit: i32,
    pub seat_limit: i32,
    pub quota_window_secs: i32,
    pub calls_per_window: i32,
    pub hosted_models_enabled: bool,
    pub promo_label: Option<String>,
    pub featured: bool,
    pub enabled: bool,
    pub sort_order: i32,
}

#[derive(Clone, Default)]
struct PlanCache {
    inner: Arc<RwLock<PlanCacheInner>>,
}

#[derive(Default)]
struct PlanCacheInner {
    by_id: HashMap<String, CloudPlanRow>,
    loaded_at: Option<Instant>,
}

const CACHE_TTL: Duration = Duration::from_secs(30);

static PLAN_CACHE: OnceLock<PlanCache> = OnceLock::new();

fn plan_cache() -> &'static PlanCache {
    PLAN_CACHE.get_or_init(PlanCache::default)
}

impl PlanCache {
    fn get(&self, plan: &str) -> Option<CloudPlanRow> {
        let guard = self.inner.read().ok()?;
        if guard.loaded_at.is_some_and(|t| t.elapsed() > CACHE_TTL) {
            return None;
        }
        guard.by_id.get(plan).cloned()
    }

    fn upsert(&self, row: CloudPlanRow) {
        if let Ok(mut guard) = self.inner.write() {
            guard.by_id.insert(row.id.clone(), row);
            if guard.loaded_at.is_none() {
                guard.loaded_at = Some(Instant::now());
            }
        }
    }

    fn replace_all(&self, rows: Vec<CloudPlanRow>) {
        if let Ok(mut guard) = self.inner.write() {
            guard.by_id = rows.into_iter().map(|r| (r.id.clone(), r)).collect();
            guard.loaded_at = Some(Instant::now());
        }
    }

    fn invalidate(&self) {
        if let Ok(mut guard) = self.inner.write() {
            guard.by_id.clear();
            guard.loaded_at = None;
        }
    }
}

/// Hardcoded fallback when DB/cache is unavailable (seed-aligned).
pub fn static_limits_for_plan(plan: &str) -> PlanLimits {
    match plan {
        "cloud_5h" => PlanLimits {
            token_limit: 1_000_000_000,
            api_key_limit: 3,
            seat_limit: 1,
            monthly_price_fen: 9_800,
            yearly_price_fen: 98_000,
            currency: "CNY",
            hosted_models_enabled: true,
            quota_window_secs: 0,
            calls_per_window: 0,
        },
        "pro" => PlanLimits {
            token_limit: 10_000_000_000,
            api_key_limit: 5,
            seat_limit: 1,
            monthly_price_fen: 59_900,
            yearly_price_fen: 599_000,
            currency: "CNY",
            hosted_models_enabled: true,
            quota_window_secs: 0,
            calls_per_window: 0,
        },
        "team" => PlanLimits {
            token_limit: 15_000_000_000,
            api_key_limit: 20,
            seat_limit: 10,
            monthly_price_fen: 199_900,
            yearly_price_fen: 1_999_000,
            currency: "CNY",
            hosted_models_enabled: true,
            quota_window_secs: 0,
            calls_per_window: 0,
        },
        _ => PlanLimits {
            token_limit: 20_000_000,
            api_key_limit: 1,
            seat_limit: 1,
            monthly_price_fen: 0,
            yearly_price_fen: 0,
            currency: "CNY",
            hosted_models_enabled: true,
            quota_window_secs: 0,
            calls_per_window: 0,
        },
    }
}

fn row_to_limits(row: &CloudPlanRow) -> PlanLimits {
    PlanLimits {
        token_limit: row.token_limit,
        api_key_limit: row.api_key_limit,
        seat_limit: row.seat_limit,
        monthly_price_fen: row.monthly_price_fen,
        yearly_price_fen: row.yearly_price_fen,
        currency: "CNY",
        hosted_models_enabled: row.hosted_models_enabled,
        quota_window_secs: row.quota_window_secs,
        calls_per_window: row.calls_per_window,
    }
}

fn map_plan_row(row: sqlx::mysql::MySqlRow) -> CloudPlanRow {
    CloudPlanRow {
        id: row.get("id"),
        display_name: row.get("display_name"),
        description: row.get("description"),
        monthly_price_fen: row.get("monthly_price_fen"),
        yearly_price_fen: row.get("yearly_price_fen"),
        token_limit: row.get("token_limit"),
        api_key_limit: row.get("api_key_limit"),
        seat_limit: row.get("seat_limit"),
        quota_window_secs: row.get("quota_window_secs"),
        calls_per_window: row.get("calls_per_window"),
        hosted_models_enabled: row.get::<i8, _>("hosted_models_enabled") != 0,
        promo_label: row.get("promo_label"),
        featured: row.get::<i8, _>("featured") != 0,
        enabled: row.get::<i8, _>("enabled") != 0,
        sort_order: row.get("sort_order"),
    }
}

pub async fn list_plans(db: &AccountDb, enabled_only: bool) -> Result<Vec<CloudPlanRow>> {
    let sql = if enabled_only {
        r#"
        SELECT id, display_name, description, monthly_price_fen, yearly_price_fen,
          token_limit, api_key_limit, seat_limit, quota_window_secs, calls_per_window,
          hosted_models_enabled, promo_label, featured, enabled, sort_order
        FROM cloud_plans WHERE enabled = 1 ORDER BY sort_order ASC, id ASC
        "#
    } else {
        r#"
        SELECT id, display_name, description, monthly_price_fen, yearly_price_fen,
          token_limit, api_key_limit, seat_limit, quota_window_secs, calls_per_window,
          hosted_models_enabled, promo_label, featured, enabled, sort_order
        FROM cloud_plans ORDER BY sort_order ASC, id ASC
        "#
    };
    let rows = sqlx::query(sql).fetch_all(db.pool()).await?;
    Ok(rows.into_iter().map(map_plan_row).collect())
}

pub async fn refresh_plan_cache(db: &AccountDb) -> Result<Vec<CloudPlanRow>> {
    let rows = list_plans(db, false).await?;
    plan_cache().replace_all(rows.clone());
    Ok(rows)
}

async fn load_plan_row(db: &AccountDb, plan: &str) -> Result<Option<CloudPlanRow>> {
    let row = sqlx::query(
        r#"
        SELECT id, display_name, description, monthly_price_fen, yearly_price_fen,
          token_limit, api_key_limit, seat_limit, quota_window_secs, calls_per_window,
          hosted_models_enabled, promo_label, featured, enabled, sort_order
        FROM cloud_plans WHERE id = ?
        "#,
    )
    .bind(plan)
    .fetch_optional(db.pool())
    .await?;
    Ok(row.map(map_plan_row))
}

/// Prefer cache → DB → static fallback.
pub async fn limits_for_plan(db: &AccountDb, plan: &str) -> PlanLimits {
    let cache = plan_cache();
    if let Some(row) = cache.get(plan) {
        return row_to_limits(&row);
    }
    match load_plan_row(db, plan).await {
        Ok(Some(row)) => {
            cache.upsert(row.clone());
            row_to_limits(&row)
        }
        _ => static_limits_for_plan(plan),
    }
}

pub async fn uses_call_quota(db: &AccountDb, plan: &str) -> bool {
    limits_for_plan(db, plan).await.calls_per_window > 0
}

pub fn subscription_status_for_upgrade(_plan: &str) -> &'static str {
    "active"
}

#[derive(Debug, Deserialize)]
pub struct PatchPlanBody {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub monthly_price_fen: Option<i32>,
    pub yearly_price_fen: Option<i32>,
    pub token_limit: Option<i64>,
    pub api_key_limit: Option<i32>,
    pub seat_limit: Option<i32>,
    pub quota_window_secs: Option<i32>,
    pub calls_per_window: Option<i32>,
    pub hosted_models_enabled: Option<bool>,
    pub promo_label: Option<Option<String>>,
    pub featured: Option<bool>,
    pub enabled: Option<bool>,
    pub sort_order: Option<i32>,
}

pub async fn patch_plan(
    db: &AccountDb,
    plan_id: &str,
    body: &PatchPlanBody,
) -> Result<CloudPlanRow> {
    let existing = load_plan_row(db, plan_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("plan not found"))?;

    let display_name = body
        .display_name
        .clone()
        .unwrap_or(existing.display_name.clone());
    let description = body.description.clone().or(existing.description.clone());
    let monthly_price_fen = body.monthly_price_fen.unwrap_or(existing.monthly_price_fen);
    let yearly_price_fen = body.yearly_price_fen.unwrap_or(existing.yearly_price_fen);
    let token_limit = body.token_limit.unwrap_or(existing.token_limit);
    let api_key_limit = body.api_key_limit.unwrap_or(existing.api_key_limit);
    let seat_limit = body.seat_limit.unwrap_or(existing.seat_limit);
    let quota_window_secs = body.quota_window_secs.unwrap_or(existing.quota_window_secs);
    let calls_per_window = body.calls_per_window.unwrap_or(existing.calls_per_window);
    let hosted_models_enabled = body
        .hosted_models_enabled
        .unwrap_or(existing.hosted_models_enabled);
    let promo_label = match &body.promo_label {
        Some(v) => v.clone(),
        None => existing.promo_label.clone(),
    };
    let featured = body.featured.unwrap_or(existing.featured);
    let enabled = body.enabled.unwrap_or(existing.enabled);
    let sort_order = body.sort_order.unwrap_or(existing.sort_order);

    sqlx::query(
        r#"
        UPDATE cloud_plans SET
          display_name = ?, description = ?,
          monthly_price_fen = ?, yearly_price_fen = ?,
          token_limit = ?, api_key_limit = ?, seat_limit = ?,
          quota_window_secs = ?, calls_per_window = ?,
          hosted_models_enabled = ?, promo_label = ?,
          featured = ?, enabled = ?, sort_order = ?,
          updated_at = NOW()
        WHERE id = ?
        "#,
    )
    .bind(&display_name)
    .bind(&description)
    .bind(monthly_price_fen)
    .bind(yearly_price_fen)
    .bind(token_limit)
    .bind(api_key_limit)
    .bind(seat_limit)
    .bind(quota_window_secs)
    .bind(calls_per_window)
    .bind(if hosted_models_enabled { 1 } else { 0 })
    .bind(&promo_label)
    .bind(if featured { 1 } else { 0 })
    .bind(if enabled { 1 } else { 0 })
    .bind(sort_order)
    .bind(plan_id)
    .execute(db.pool())
    .await?;

    plan_cache().invalidate();
    let _ = refresh_plan_cache(db).await?;
    load_plan_row(db, plan_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("plan missing after update"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_plan_limits() {
        let l = static_limits_for_plan("free");
        assert_eq!(l.token_limit, 20_000_000);
        assert!(l.hosted_models_enabled);
        assert_eq!(l.calls_per_window, 0);
    }

    #[test]
    fn cloud_5h_quota() {
        let l = static_limits_for_plan("cloud_5h");
        assert_eq!(l.token_limit, 1_000_000_000);
        assert_eq!(l.monthly_price_fen, 9_800);
        assert_eq!(l.quota_window_secs, 0);
        assert_eq!(l.calls_per_window, 0);
    }

    #[test]
    fn pro_plan_limits() {
        let l = static_limits_for_plan("pro");
        assert_eq!(l.api_key_limit, 5);
        assert_eq!(l.monthly_price_fen, 59_900);
        assert_eq!(l.token_limit, 10_000_000_000);
        assert_eq!(l.currency, "CNY");
        assert_eq!(l.quota_window_secs, 0);
        assert_eq!(l.calls_per_window, 0);
    }
}
