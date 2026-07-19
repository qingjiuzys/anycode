use crate::db::AccountDb;
use crate::models::{CloudModelView, UsageByModelView, UsageSummaryView};
use anyhow::{anyhow, Result};
use sqlx::Row;
use uuid::Uuid;

/// Hosted named models exposed to clients (excluding synthetic `auto`).
pub const ALLOWED_HOSTED_MODEL_IDS: &[&str] =
    &["deepseek-v4-flash", "deepseek-v4-pro", "agnes-chat"];

pub fn is_allowed_hosted_model(model_id: &str) -> bool {
    model_id == "auto" || ALLOWED_HOSTED_MODEL_IDS.contains(&model_id)
}

pub async fn list_models(db: &AccountDb, plan_tier: &str) -> Result<Vec<CloudModelView>> {
    let rows = sqlx::query(
        r#"
        SELECT id, provider_id, display_name, context_window,
               CAST(COALESCE(price_per_1m_input_cny, 0) AS DOUBLE)
                 AS price_per_1m_input_cny,
               CAST(COALESCE(price_per_1m_output_cny, 0) AS DOUBLE)
                 AS price_per_1m_output_cny,
               currency, min_plan
        FROM cloud_models WHERE enabled = 1 ORDER BY sort_order ASC, display_name ASC
        "#,
    )
    .fetch_all(db.pool())
    .await?;

    let mut out: Vec<CloudModelView> = rows
        .into_iter()
        .filter(|r| {
            let id: String = r.get("id");
            ALLOWED_HOSTED_MODEL_IDS.contains(&id.as_str())
        })
        .map(|r| {
            let min_plan: String = r.get("min_plan");
            CloudModelView {
                id: r.get("id"),
                provider_id: r.get("provider_id"),
                display_name: r.get("display_name"),
                context_window: r.get("context_window"),
                price_per_1m_input_cny: r.get("price_per_1m_input_cny"),
                price_per_1m_output_cny: r.get("price_per_1m_output_cny"),
                currency: r.get("currency"),
                min_plan: min_plan.clone(),
                available: plan_allows_model(plan_tier, &min_plan),
            }
        })
        .collect();
    let has_available = out.iter().any(|m| m.available);
    if has_available {
        out.insert(0, cloud_auto_model_view(plan_tier));
    }
    Ok(out)
}

fn plan_allows_model(user_plan: &str, min_plan: &str) -> bool {
    let rank = |p: &str| match p {
        "team" => 3,
        "pro" | "cloud_5h" => 2,
        _ => 1,
    };
    rank(user_plan) >= rank(min_plan)
}

/// Resolve `auto` to DeepSeek Flash when plan-available (cost-first); else Agnes.
pub async fn resolve_model_id(db: &AccountDb, plan_tier: &str, model_id: &str) -> Result<String> {
    if model_id == "auto" {
        let models = list_models(db, plan_tier).await?;
        if let Some(flash) = models
            .iter()
            .find(|m| m.id == "deepseek-v4-flash" && m.available)
        {
            return Ok(flash.id.clone());
        }
        let chat = models
            .iter()
            .find(|m| m.id == "agnes-chat" && m.available)
            .ok_or_else(|| anyhow!("no hosted chat model available for plan"))?;
        return Ok(chat.id.clone());
    }
    if !is_allowed_hosted_model(model_id) {
        return Err(anyhow!("model not supported: {model_id}"));
    }
    let models = list_models(db, plan_tier).await?;
    if !models.iter().any(|m| m.id == model_id && m.available) {
        return Err(anyhow!("model not available for plan"));
    }
    Ok(model_id.to_string())
}

pub async fn get_model_upstream(
    db: &AccountDb,
    model_id: &str,
) -> Result<Option<(String, String)>> {
    if model_id == "auto" {
        return Ok(None);
    }
    if !is_allowed_hosted_model(model_id) {
        return Err(anyhow!("model not supported: {model_id}"));
    }
    let row = sqlx::query(
        "SELECT provider_id, upstream_model FROM cloud_models WHERE id = ? AND enabled = 1",
    )
    .bind(model_id)
    .fetch_optional(db.pool())
    .await?;
    Ok(row.map(|r| (r.get("provider_id"), r.get("upstream_model"))))
}

pub fn cloud_auto_model_view(plan_tier: &str) -> CloudModelView {
    CloudModelView {
        id: "auto".into(),
        provider_id: "anycode_cloud".into(),
        display_name: "Cloud Auto".into(),
        context_window: 128_000,
        price_per_1m_input_cny: 0.0,
        price_per_1m_output_cny: 0.0,
        currency: "CNY".into(),
        min_plan: "free".into(),
        available: plan_allows_model(plan_tier, "free"),
    }
}

pub async fn usage_summary(db: &AccountDb, org_id: &str) -> Result<UsageSummaryView> {
    let ent = crate::store::get_entitlements(db, org_id).await?;
    let rows = sqlx::query(
        r#"
        SELECT model_id, CAST(SUM(total_tokens) AS SIGNED) AS total_tokens
        FROM usage_events WHERE organization_id = ?
        GROUP BY model_id ORDER BY total_tokens DESC
        "#,
    )
    .bind(org_id)
    .fetch_all(db.pool())
    .await?;

    let by_model = rows
        .into_iter()
        .map(|r| UsageByModelView {
            model_id: r.get("model_id"),
            total_tokens: r.get("total_tokens"),
        })
        .collect();

    Ok(UsageSummaryView {
        tokens_used: ent.tokens_used,
        token_limit: ent.token_limit,
        by_model,
    })
}

pub async fn record_usage(
    db: &AccountDb,
    org_id: &str,
    model_id: &str,
    prompt_tokens: i64,
    completion_tokens: i64,
    upstream_account_id: Option<&str>,
) -> Result<()> {
    crate::quota::record_model_call(db, org_id).await?;
    let total = prompt_tokens + completion_tokens;
    let id = format!("use_{}", Uuid::new_v4());
    let mut tx = db.pool().begin().await?;
    sqlx::query(
        r#"
        INSERT INTO usage_events (id, organization_id, model_id, prompt_tokens, completion_tokens, total_tokens, upstream_account_id)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(org_id)
    .bind(model_id)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(total)
    .bind(upstream_account_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE entitlements SET tokens_used = tokens_used + ?, updated_at = NOW() WHERE organization_id = ?",
    )
    .bind(total)
    .bind(org_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn check_quota(db: &AccountDb, org_id: &str, needed_tokens: i64) -> Result<bool> {
    let row = sqlx::query(
        "SELECT token_limit, tokens_used, hosted_models_enabled FROM entitlements WHERE organization_id = ?",
    )
    .bind(org_id)
    .fetch_one(db.pool())
    .await?;
    let hosted: bool = row.get("hosted_models_enabled");
    if !hosted {
        return Ok(false);
    }
    let limit: i64 = row.get("token_limit");
    let used: i64 = row.get("tokens_used");
    Ok(used + needed_tokens <= limit)
}

pub async fn resolve_org_by_api_key(db: &AccountDb, api_key: &str) -> Result<Option<String>> {
    let row = sqlx::query(
        r#"
        SELECT organization_id FROM cloud_api_keys
        WHERE token_hash = ? AND revoked_at IS NULL
          AND (expires_at IS NULL OR expires_at > NOW())
        "#,
    )
    .bind(crate::auth::hash_token(api_key))
    .fetch_optional(db.pool())
    .await?;
    Ok(row.map(|r| r.get("organization_id")))
}

pub async fn resolve_org_by_session(db: &AccountDb, token: &str) -> Result<Option<String>> {
    let row = sqlx::query(
        r#"
        SELECT u.organization_id
        FROM sessions s JOIN users u ON u.id = s.user_id
        WHERE s.token_hash = ? AND s.expires_at > NOW() AND u.status = 'active'
        "#,
    )
    .bind(crate::auth::hash_token(token))
    .fetch_optional(db.pool())
    .await?;
    Ok(row.map(|r| r.get("organization_id")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_hosted_models_contract() {
        assert!(is_allowed_hosted_model("auto"));
        assert!(is_allowed_hosted_model("agnes-chat"));
        assert!(is_allowed_hosted_model("deepseek-v4-flash"));
        assert!(is_allowed_hosted_model("deepseek-v4-pro"));
        assert!(!is_allowed_hosted_model("agnes-code"));
        assert!(!is_allowed_hosted_model("agnes-reasoner"));
    }
}
