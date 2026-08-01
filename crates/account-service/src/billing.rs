//! Shared billing helpers: prepaid activation, subscription expiry, order records.

use crate::db::AccountDb;
use crate::plan::limits_for_plan;
use anyhow::{anyhow, Result};
use chrono::{Datelike, NaiveDate, Utc};
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize)]
pub struct PaymentOrderView {
    pub id: String,
    pub provider: String,
    pub plan: String,
    pub billing_cycle: String,
    pub amount_fen: i32,
    pub currency: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_trade_no: Option<String>,
    pub code_url: Option<String>,
    pub expires_at: String,
    pub paid_at: Option<String>,
}

pub async fn refresh_subscription_status(db: &AccountDb, org_id: &str) -> Result<()> {
    let row = sqlx::query(
        r#"
        SELECT plan, status, period_end, payment_provider
        FROM subscriptions WHERE organization_id = ?
        "#,
    )
    .bind(org_id)
    .fetch_optional(db.pool())
    .await?;
    let Some(row) = row else {
        return Ok(());
    };
    let plan: String = row.get("plan");
    let status: String = row.get("status");
    let period_end: NaiveDate = row.get("period_end");
    let provider: Option<String> = row.get("payment_provider");
    let today = Utc::now().date_naive();
    if plan != "free"
        && status == "active"
        && period_end < today
        && provider.as_deref() == Some("wechat")
    {
        downgrade_expired_prepaid(db, org_id).await?;
    } else if plan == "free" && period_end < today {
        // 免费订阅账期随当前自然月滚动，避免 UI 一直显示过期周期（剩余 0 天）。
        let (period_start, period_end) = current_calendar_month();
        sqlx::query(
            "UPDATE subscriptions SET period_start = ?, period_end = ?, updated_at = NOW() WHERE organization_id = ?",
        )
        .bind(period_start)
        .bind(period_end)
        .bind(org_id)
        .execute(db.pool())
        .await?;
    }
    Ok(())
}

async fn downgrade_expired_prepaid(db: &AccountDb, org_id: &str) -> Result<()> {
    let limits = limits_for_plan(db, "free").await;
    let mut tx = db.pool().begin().await?;
    sqlx::query("UPDATE organizations SET plan_tier = 'free', updated_at = NOW() WHERE id = ?")
        .bind(org_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE subscriptions SET plan = 'free', status = 'active', payment_method_bound = 0, pass_expires_at = NULL, updated_at = NOW() WHERE organization_id = ?",
    )
    .bind(org_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        UPDATE entitlements SET token_limit = ?, api_key_limit = ?, seat_limit = ?,
          hosted_models_enabled = 0, cloud_unlimited_rate = 0,
          quota_window_secs = 0, calls_limit_per_window = 0, calls_used_in_window = 0,
          quota_window_started_at = NULL, updated_at = NOW()
        WHERE organization_id = ?
        "#,
    )
    .bind(limits.token_limit)
    .bind(limits.api_key_limit)
    .bind(limits.seat_limit)
    .bind(org_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct PrepaidActivation {
    pub org_id: String,
    pub plan: String,
    pub billing_cycle: String,
    pub payment_provider: String,
    pub payment_order_id: String,
    pub amount_fen: i32,
    pub currency: String,
}

/// Extend or start a prepaid subscription period after successful payment.
pub async fn activate_prepaid_period(db: &AccountDb, input: &PrepaidActivation) -> Result<()> {
    let org_id = &input.org_id;
    let plan = input.plan.as_str();
    let billing_cycle = input.billing_cycle.as_str();
    let payment_provider = input.payment_provider.as_str();
    let payment_order_id = input.payment_order_id.as_str();
    let amount_fen = input.amount_fen;
    let currency = input.currency.as_str();
    if currency != "CNY" {
        return Err(anyhow!("only CNY billing is supported"));
    }
    if !matches!(plan, "pro" | "team" | "cloud_5h") {
        return Err(anyhow!("invalid plan for prepaid activation"));
    }
    let limits = limits_for_plan(db, plan).await;
    let today = Utc::now().date_naive();

    let months = match billing_cycle {
        "monthly" => 1,
        "yearly" => 12,
        _ => return Err(anyhow!("invalid billing cycle")),
    };
    let current_end: NaiveDate =
        sqlx::query_scalar("SELECT period_end FROM subscriptions WHERE organization_id = ?")
            .bind(org_id)
            .fetch_one(db.pool())
            .await?;
    let period_start = if current_end >= today {
        current_end
    } else {
        today
    };
    let period_end = add_months(period_start, months)?;

    let invoice_id = format!("inv_{}", Uuid::new_v4());
    let invoice_number = format!(
        "AC-{}-{}",
        Utc::now().format("%Y%m"),
        &payment_order_id[4..12]
    );
    let mut tx = db.pool().begin().await?;
    sqlx::query("UPDATE organizations SET plan_tier = ?, updated_at = NOW() WHERE id = ?")
        .bind(plan)
        .bind(org_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        r#"
        UPDATE subscriptions SET plan = ?, status = 'active', billing_cycle = ?,
          period_start = ?, period_end = ?, prepaid_until = ?,
          payment_method_bound = 1, payment_provider = ?, pass_expires_at = NULL,
          updated_at = NOW()
        WHERE organization_id = ?
        "#,
    )
    .bind(plan)
    .bind(billing_cycle)
    .bind(period_start)
    .bind(period_end)
    .bind(payment_provider)
    .bind(org_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        UPDATE entitlements SET token_limit = ?, api_key_limit = ?, seat_limit = ?,
          hosted_models_enabled = ?, cloud_unlimited_rate = 0,
          quota_window_secs = ?, calls_limit_per_window = ?, calls_used_in_window = 0,
          quota_window_started_at = NOW(), updated_at = NOW()
        WHERE organization_id = ?
        "#,
    )
    .bind(limits.token_limit)
    .bind(limits.api_key_limit)
    .bind(limits.seat_limit)
    .bind(limits.hosted_models_enabled)
    .bind(limits.quota_window_secs)
    .bind(limits.calls_per_window)
    .bind(org_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE payment_orders SET status = 'paid', paid_at = NOW() WHERE id = ? AND organization_id = ?",
    )
    .bind(payment_order_id)
    .bind(org_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO invoices (id, organization_id, number, period_start, period_end,
          amount_fen, currency, amount_cny, status, payment_order_id)
        VALUES (?, ?, ?, ?, ?, ?, 'CNY', ?, 'paid', ?)
        "#,
    )
    .bind(&invoice_id)
    .bind(org_id)
    .bind(&invoice_number)
    .bind(period_start)
    .bind(period_end)
    .bind(amount_fen)
    .bind(f64::from(amount_fen) / 100.0)
    .bind(payment_order_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Stripe recurring subscription activation (unchanged semantics).
pub async fn activate_stripe_subscription(
    db: &AccountDb,
    org_id: &str,
    plan: &str,
    customer_id: &str,
    stripe_sub_id: &str,
) -> Result<()> {
    let limits = limits_for_plan(db, plan).await;
    let (period_start, period_end) = current_calendar_month();
    let mut tx = db.pool().begin().await?;
    sqlx::query("UPDATE organizations SET plan_tier = ?, updated_at = NOW() WHERE id = ?")
        .bind(plan)
        .bind(org_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        r#"
        UPDATE subscriptions SET plan = ?, status = 'active', payment_method_bound = 1,
          payment_provider = 'stripe', stripe_customer_id = ?, stripe_subscription_id = ?,
          period_start = ?, period_end = ?, updated_at = NOW()
        WHERE organization_id = ?
        "#,
    )
    .bind(plan)
    .bind(customer_id)
    .bind(stripe_sub_id)
    .bind(period_start)
    .bind(period_end)
    .bind(org_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        UPDATE entitlements SET token_limit = ?, api_key_limit = ?, seat_limit = ?,
          hosted_models_enabled = ?, cloud_unlimited_rate = 0,
          quota_window_secs = ?, calls_limit_per_window = ?, calls_used_in_window = 0,
          quota_window_started_at = NULL, updated_at = NOW() WHERE organization_id = ?
        "#,
    )
    .bind(limits.token_limit)
    .bind(limits.api_key_limit)
    .bind(limits.seat_limit)
    .bind(limits.hosted_models_enabled)
    .bind(limits.quota_window_secs)
    .bind(limits.calls_per_window)
    .bind(org_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn get_payment_order(
    db: &AccountDb,
    org_id: &str,
    order_id: &str,
) -> Result<Option<PaymentOrderView>> {
    let row = sqlx::query(
        r#"
        SELECT id, provider, plan, billing_cycle, COALESCE(amount_fen, amount_cents) AS amount_fen,
          currency, status, out_trade_no,
          code_url, expires_at, paid_at
        FROM payment_orders
        WHERE id = ? AND organization_id = ?
        "#,
    )
    .bind(order_id)
    .bind(org_id)
    .fetch_optional(db.pool())
    .await?;
    Ok(row.map(|r| {
        let expires: chrono::DateTime<Utc> = r.get("expires_at");
        let paid: Option<chrono::DateTime<Utc>> = r.get("paid_at");
        PaymentOrderView {
            id: r.get("id"),
            provider: r.get("provider"),
            plan: r.get("plan"),
            billing_cycle: r.get("billing_cycle"),
            amount_fen: r.get("amount_fen"),
            currency: r.get("currency"),
            status: r.get("status"),
            out_trade_no: r.get("out_trade_no"),
            code_url: r.get("code_url"),
            expires_at: expires.to_rfc3339(),
            paid_at: paid.map(|t| t.to_rfc3339()),
        }
    }))
}

#[derive(Debug, Clone)]
pub struct PendingOrderInput {
    pub org_id: String,
    pub provider: String,
    pub plan: String,
    pub billing_cycle: String,
    pub amount_fen: i32,
    pub currency: String,
    pub out_trade_no: String,
    pub code_url: Option<String>,
    pub expires_at: chrono::DateTime<Utc>,
}

pub async fn insert_pending_order(db: &AccountDb, input: &PendingOrderInput) -> Result<String> {
    let id = format!("pay_{}", Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO payment_orders (
          id, organization_id, provider, plan, billing_cycle, amount_fen, amount_cents, currency,
          status, out_trade_no, code_url, expires_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&input.org_id)
    .bind(&input.provider)
    .bind(&input.plan)
    .bind(&input.billing_cycle)
    .bind(input.amount_fen)
    .bind(input.amount_fen)
    .bind(&input.currency)
    .bind(&input.out_trade_no)
    .bind(&input.code_url)
    .bind(input.expires_at)
    .execute(db.pool())
    .await?;
    Ok(id)
}

/// Amount/currency for a pending order (WeChat notify validation).
pub async fn pending_order_amount_by_out_trade_no(
    db: &AccountDb,
    out_trade_no: &str,
) -> Result<Option<(i32, String)>> {
    let row = sqlx::query(
        r#"
        SELECT COALESCE(amount_fen, amount_cents) AS amount_fen, currency
        FROM payment_orders
        WHERE out_trade_no = ? AND status = 'pending'
        "#,
    )
    .bind(out_trade_no)
    .fetch_optional(db.pool())
    .await?;
    Ok(row.map(|r| {
        let amount_fen: i32 = r.get("amount_fen");
        let currency: String = r.get("currency");
        (amount_fen, currency)
    }))
}

pub async fn mark_order_paid_by_out_trade_no(
    db: &AccountDb,
    out_trade_no: &str,
    provider_trade_no: &str,
) -> Result<()> {
    let existing: Option<String> =
        sqlx::query_scalar("SELECT status FROM payment_orders WHERE out_trade_no = ?")
            .bind(out_trade_no)
            .fetch_optional(db.pool())
            .await?;
    if existing.as_deref() == Some("paid") {
        return Ok(());
    }

    let row = sqlx::query(
        r#"
        SELECT id, organization_id, plan, billing_cycle,
          COALESCE(amount_fen, amount_cents) AS amount_fen, currency
        FROM payment_orders
        WHERE out_trade_no = ? AND status = 'pending'
        "#,
    )
    .bind(out_trade_no)
    .fetch_optional(db.pool())
    .await?;
    let Some(row) = row else {
        return Ok(());
    };
    let order_id: String = row.get("id");
    let org_id: String = row.get("organization_id");
    let plan: String = row.get("plan");
    let billing_cycle: String = row.get("billing_cycle");
    let amount_fen: i32 = row.get("amount_fen");
    let currency: String = row.get("currency");

    sqlx::query(
        "UPDATE payment_orders SET provider_trade_no = ? WHERE id = ? AND status = 'pending'",
    )
    .bind(provider_trade_no)
    .bind(&order_id)
    .execute(db.pool())
    .await?;

    activate_prepaid_period(
        db,
        &PrepaidActivation {
            org_id: org_id.clone(),
            plan,
            billing_cycle,
            payment_provider: "wechat".into(),
            payment_order_id: order_id,
            amount_fen,
            currency,
        },
    )
    .await?;
    Ok(())
}

fn add_months(date: NaiveDate, months: i32) -> Result<NaiveDate> {
    let mut y = date.year();
    let mut m = date.month() as i32 + months;
    while m > 12 {
        m -= 12;
        y += 1;
    }
    while m < 1 {
        m += 12;
        y -= 1;
    }
    let day = date.day().min(days_in_month(y, m as u32));
    NaiveDate::from_ymd_opt(y, m as u32, day).ok_or_else(|| anyhow!("invalid date"))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    NaiveDate::from_ymd_opt(
        if month == 12 { year + 1 } else { year },
        if month == 12 { 1 } else { month + 1 },
        1,
    )
    .and_then(|d| d.pred_opt())
    .map(|d| d.day())
    .unwrap_or(28)
}

fn current_calendar_month() -> (NaiveDate, NaiveDate) {
    let today = Utc::now().date_naive();
    let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
    let end = add_months(start, 1)
        .unwrap_or(today)
        .pred_opt()
        .unwrap_or(today);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::add_months;
    use chrono::NaiveDate;

    #[test]
    fn add_months_handles_year_rollover() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 15).unwrap();
        let end = add_months(d, 2).unwrap();
        assert_eq!(end, NaiveDate::from_ymd_opt(2027, 1, 15).unwrap());
    }
}
