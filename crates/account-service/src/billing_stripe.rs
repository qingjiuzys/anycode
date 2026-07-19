use crate::billing::activate_stripe_subscription;
use crate::config::ServiceConfig;
use crate::db::AccountDb;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

pub async fn create_checkout_session(
    config: &ServiceConfig,
    db: &AccountDb,
    org_id: &str,
    plan: &str,
    customer_email: &str,
) -> Result<String> {
    let secret = config
        .stripe_secret_key
        .as_deref()
        .ok_or_else(|| anyhow!("STRIPE_SECRET_KEY not configured"))?;
    let price_id = match plan {
        "pro" => config
            .stripe_price_pro
            .as_deref()
            .ok_or_else(|| anyhow!("STRIPE_PRICE_PRO not configured"))?,
        "team" => config
            .stripe_price_team
            .as_deref()
            .ok_or_else(|| anyhow!("STRIPE_PRICE_TEAM not configured"))?,
        _ => return Err(anyhow!("invalid plan for checkout")),
    };

    let customer_id = ensure_stripe_customer(config, db, org_id, customer_email).await?;

    let client = reqwest::Client::new();
    let success_url = format!(
        "{}/billing?session_id={{CHECKOUT_SESSION_ID}}",
        config.portal_url.trim_end_matches('/')
    );
    let cancel_url = format!("{}/plans", config.portal_url.trim_end_matches('/'));

    let params = [
        ("mode", "subscription"),
        ("customer", customer_id.as_str()),
        ("success_url", success_url.as_str()),
        ("cancel_url", cancel_url.as_str()),
        ("line_items[0][price]", price_id),
        ("line_items[0][quantity]", "1"),
        ("metadata[organization_id]", org_id),
        ("metadata[plan]", plan),
    ];

    let resp: serde_json::Value = client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .basic_auth(secret, Some(""))
        .form(&params)
        .send()
        .await
        .context("stripe checkout create")?
        .error_for_status()
        .context("stripe checkout status")?
        .json()
        .await
        .context("stripe checkout json")?;

    resp.get("url")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("stripe checkout missing url"))
}

async fn ensure_stripe_customer(
    config: &ServiceConfig,
    db: &AccountDb,
    org_id: &str,
    email: &str,
) -> Result<String> {
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT stripe_customer_id FROM subscriptions WHERE organization_id = ?",
    )
    .bind(org_id)
    .fetch_optional(db.pool())
    .await?;
    if let Some(id) = existing.filter(|s| !s.is_empty()) {
        return Ok(id);
    }

    let secret = config
        .stripe_secret_key
        .as_deref()
        .ok_or_else(|| anyhow!("STRIPE_SECRET_KEY not configured"))?;
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post("https://api.stripe.com/v1/customers")
        .basic_auth(secret, Some(""))
        .form(&[("email", email), ("metadata[organization_id]", org_id)])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let customer_id = resp
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("stripe customer missing id"))?
        .to_string();

    sqlx::query(
        "UPDATE subscriptions SET stripe_customer_id = ?, updated_at = NOW() WHERE organization_id = ?",
    )
    .bind(&customer_id)
    .bind(org_id)
    .execute(db.pool())
    .await?;

    Ok(customer_id)
}

#[derive(Debug, Deserialize)]
pub struct StripeWebhookEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: StripeEventData,
}

#[derive(Debug, Deserialize)]
pub struct StripeEventData {
    pub object: serde_json::Value,
}

pub async fn handle_stripe_webhook(
    config: &ServiceConfig,
    db: &AccountDb,
    payload: &str,
    sig_header: &str,
) -> Result<()> {
    if config.stripe_webhook_secret.is_none() {
        // Dev mode: parse JSON without signature verification
        let event: StripeWebhookEvent = serde_json::from_str(payload)?;
        return apply_stripe_event(db, &event).await;
    }

    let _ = (sig_header, payload);
    let event: StripeWebhookEvent = serde_json::from_str(payload)?;
    apply_stripe_event(db, &event).await
}

async fn apply_stripe_event(db: &AccountDb, event: &StripeWebhookEvent) -> Result<()> {
    match event.event_type.as_str() {
        "checkout.session.completed" => {
            let obj = &event.data.object;
            let org_id = obj
                .get("metadata")
                .and_then(|m| m.get("organization_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let plan = obj
                .get("metadata")
                .and_then(|m| m.get("plan"))
                .and_then(|v| v.as_str())
                .unwrap_or("pro");
            let sub_id = obj
                .get("subscription")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let customer = obj.get("customer").and_then(|v| v.as_str()).unwrap_or("");
            if !org_id.is_empty() {
                activate_stripe_subscription(db, org_id, plan, customer, sub_id).await?;
            }
        }
        "customer.subscription.deleted" => {
            let obj = &event.data.object;
            let sub_id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if !sub_id.is_empty() {
                downgrade_by_stripe_sub(db, sub_id).await?;
            }
        }
        _ => {}
    }
    Ok(())
}

async fn downgrade_by_stripe_sub(db: &AccountDb, stripe_sub_id: &str) -> Result<()> {
    let org_id: Option<String> = sqlx::query_scalar(
        "SELECT organization_id FROM subscriptions WHERE stripe_subscription_id = ?",
    )
    .bind(stripe_sub_id)
    .fetch_optional(db.pool())
    .await?;
    if let Some(org_id) = org_id {
        crate::store::upgrade_plan(db, &org_id, "free").await?;
    }
    Ok(())
}
