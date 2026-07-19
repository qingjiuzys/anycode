use crate::auth::{
    hash_password, hash_token, new_api_key_plaintext, new_session_token, password_needs_rehash,
    verify_password,
};
use crate::config::ServiceConfig;
use crate::db::AccountDb;
use crate::models::{
    AccountBundle, AuthUser, BillingContactView, CloudApiKeyView, EntitlementsView, InvoiceView,
    OrgMemberView, OrganizationSummary, SubscriptionView,
};
use crate::plan::{limits_for_plan, subscription_status_for_upgrade};
use anyhow::{anyhow, Result};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use sqlx::Row;
use uuid::Uuid;

pub struct RegisterInput {
    pub email: String,
    pub password: String,
    pub display_name: String,
    pub verification_code: String,
    pub privacy_consent: bool,
    pub consent_version: String,
}

pub async fn register(
    db: &AccountDb,
    config: &ServiceConfig,
    input: RegisterInput,
) -> Result<(AuthUser, String)> {
    let email = input.email.trim().to_lowercase();
    if email.is_empty() || input.password.len() < 8 {
        return Err(anyhow!("email and password (min 8 chars) required"));
    }
    if !input.privacy_consent || input.consent_version.trim().is_empty() {
        return Err(anyhow!("privacy consent is required"));
    }
    if !crate::email_verification::consume_registration_code(
        db,
        config,
        &email,
        &input.verification_code,
    )
    .await?
    {
        return Err(anyhow!("invalid or expired verification code"));
    }
    let existing: Option<i64> = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email = ?")
        .bind(&email)
        .fetch_one(db.pool())
        .await?;
    if existing.unwrap_or(0) > 0 {
        return Err(anyhow!("email already registered"));
    }

    let org_id = format!("org_{}", Uuid::new_v4());
    let user_id = format!("usr_{}", Uuid::new_v4());
    let org_name = if input.display_name.trim().is_empty() {
        email.split('@').next().unwrap_or("Workspace").to_string()
    } else {
        format!("{} workspace", input.display_name.trim())
    };
    let (period_start, period_end) = current_billing_period();
    let limits = limits_for_plan(db, "free").await;
    let invoice_id = format!("inv_{}", Uuid::new_v4());

    let mut tx = db.pool().begin().await?;
    sqlx::query(
        "INSERT INTO organizations (id, name, plan_tier, sso_status) VALUES (?, ?, 'free', 'disabled')",
    )
    .bind(&org_id)
    .bind(&org_name)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO users
          (id, organization_id, email, display_name, role, password_hash, status, email_verified_at, identity_status)
        VALUES (?, ?, ?, ?, 'owner', ?, 'identity_pending', NOW(), 'identity_pending')
        "#,
    )
    .bind(&user_id)
    .bind(&org_id)
    .bind(&email)
    .bind(input.display_name.trim())
    .bind(hash_password(&input.password))
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO user_consents (id, user_id, consent_type, policy_version) VALUES (?, ?, 'privacy', ?)",
    )
    .bind(format!("cons_{}", Uuid::new_v4()))
    .bind(&user_id)
    .bind(input.consent_version.trim())
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO subscriptions (organization_id, plan, status, billing_cycle, period_start, period_end)
        VALUES (?, 'free', 'active', 'monthly', ?, ?)
        "#,
    )
    .bind(&org_id)
    .bind(period_start)
    .bind(period_end)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO entitlements (organization_id, token_limit, api_key_limit, seat_limit, hosted_models_enabled) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&org_id)
    .bind(limits.token_limit)
    .bind(limits.api_key_limit)
    .bind(limits.seat_limit)
    .bind(limits.hosted_models_enabled)
    .execute(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO billing_contacts (organization_id) VALUES (?)")
        .bind(&org_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO invoices (
          id, organization_id, number, period_start, period_end, amount_fen, currency, status
        ) VALUES (?, ?, ?, ?, ?, 0, 'CNY', 'paid')
        "#,
    )
    .bind(&invoice_id)
    .bind(&org_id)
    .bind(format!("AC-{}-0001", Utc::now().format("%Y%m")))
    .bind(period_start)
    .bind(period_end)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let user = get_user_by_id(db, &user_id)
        .await?
        .ok_or_else(|| anyhow!("user missing"))?;
    let token = create_session(db, &user_id).await?;
    Ok((user, token))
}

/// Local dev: seed the first portal user when `users` is empty.
pub async fn bootstrap_portal_user_if_needed(
    db: &AccountDb,
    email: Option<&str>,
    password: Option<&str>,
) -> Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(db.pool())
        .await?;
    if count > 0 {
        return Ok(());
    }
    let Some(email) = email.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    let Some(password) = password.filter(|s| s.len() >= 8) else {
        return Ok(());
    };
    let email = email.to_lowercase();
    let org_id = format!("org_{}", Uuid::new_v4());
    let user_id = format!("usr_{}", Uuid::new_v4());
    let org_name = format!("{} workspace", email.split('@').next().unwrap_or("dev"));
    let (period_start, period_end) = current_billing_period();
    let limits = limits_for_plan(db, "free").await;
    let invoice_id = format!("inv_{}", Uuid::new_v4());

    let mut tx = db.pool().begin().await?;
    sqlx::query(
        "INSERT INTO organizations (id, name, plan_tier, sso_status) VALUES (?, ?, 'free', 'disabled')",
    )
    .bind(&org_id)
    .bind(&org_name)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO users
          (id, organization_id, email, display_name, role, password_hash, status, email_verified_at, identity_status)
        VALUES (?, ?, ?, ?, 'owner', ?, 'active', NOW(), 'approved')
        "#,
    )
    .bind(&user_id)
    .bind(&org_id)
    .bind(&email)
    .bind(email.split('@').next().unwrap_or("dev"))
    .bind(hash_password(password))
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO user_consents (id, user_id, consent_type, policy_version) VALUES (?, ?, 'privacy', ?)",
    )
    .bind(format!("cons_{}", Uuid::new_v4()))
    .bind(&user_id)
    .bind("local-dev")
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO subscriptions (organization_id, plan, status, billing_cycle, period_start, period_end)
        VALUES (?, 'free', 'active', 'monthly', ?, ?)
        "#,
    )
    .bind(&org_id)
    .bind(period_start)
    .bind(period_end)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO entitlements (organization_id, token_limit, api_key_limit, seat_limit, hosted_models_enabled) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&org_id)
    .bind(limits.token_limit)
    .bind(limits.api_key_limit)
    .bind(limits.seat_limit)
    .bind(limits.hosted_models_enabled)
    .execute(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO billing_contacts (organization_id) VALUES (?)")
        .bind(&org_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO invoices (
          id, organization_id, number, period_start, period_end, amount_fen, currency, status
        ) VALUES (?, ?, ?, ?, ?, 0, 'CNY', 'paid')
        "#,
    )
    .bind(&invoice_id)
    .bind(&org_id)
    .bind(format!("AC-{}-0001", Utc::now().format("%Y%m")))
    .bind(period_start)
    .bind(period_end)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    tracing::info!("bootstrapped portal user {email}");
    Ok(())
}

pub async fn login(
    db: &AccountDb,
    email: &str,
    password: &str,
) -> Result<Option<(AuthUser, String)>> {
    let email = email.trim().to_lowercase();
    let row = sqlx::query(
        r#"
        SELECT id, organization_id, email, display_name, role, password_hash
        FROM users WHERE email = ? AND status IN ('active', 'identity_pending')
        "#,
    )
    .bind(&email)
    .fetch_optional(db.pool())
    .await?;
    let Some(r) = row else {
        return Ok(None);
    };
    let hash: String = r.get("password_hash");
    if !verify_password(password, &hash) {
        return Ok(None);
    }
    let user_id: String = r.get("id");
    if password_needs_rehash(&hash) {
        sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
            .bind(hash_password(password))
            .bind(&user_id)
            .execute(db.pool())
            .await?;
    }
    let _ = sqlx::query("UPDATE users SET last_active_at = NOW() WHERE id = ?")
        .bind(&user_id)
        .execute(db.pool())
        .await;
    let user = AuthUser {
        id: user_id.clone(),
        email: r.get("email"),
        display_name: r.get("display_name"),
        role: r.get("role"),
        organization_id: r.get("organization_id"),
    };
    let token = create_session(db, &user_id).await?;
    Ok(Some((user, token)))
}

pub async fn logout(db: &AccountDb, token: &str) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
        .bind(hash_token(token))
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn resolve_session(db: &AccountDb, token: &str) -> Result<Option<AuthUser>> {
    let row = sqlx::query(
        r#"
        SELECT u.id, u.organization_id, u.email, u.display_name, u.role
        FROM sessions s
        JOIN users u ON u.id = s.user_id
        WHERE s.token_hash = ? AND s.expires_at > NOW() AND s.revoked_at IS NULL
          AND u.status IN ('active', 'identity_pending')
        "#,
    )
    .bind(hash_token(token))
    .fetch_optional(db.pool())
    .await?;
    Ok(row.map(|r| AuthUser {
        id: r.get("id"),
        email: r.get("email"),
        display_name: r.get("display_name"),
        role: r.get("role"),
        organization_id: r.get("organization_id"),
    }))
}

pub async fn organization_has_verified_identity(db: &AccountDb, org_id: &str) -> Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE organization_id = ? AND email_verified_at IS NOT NULL AND identity_status = 'approved' AND status = 'active'",
    )
    .bind(org_id)
    .fetch_one(db.pool())
    .await?;
    Ok(count > 0)
}

pub async fn get_account_bundle(
    db: &AccountDb,
    org_id: &str,
    user: &AuthUser,
) -> Result<AccountBundle> {
    ensure_org_defaults(db, org_id).await?;
    let organization = get_organization(db, org_id).await?;
    let subscription = get_subscription(db, org_id).await?;
    let entitlements = get_entitlements(db, org_id).await?;
    let billing_contact = get_billing_contact(db, org_id, &user.email).await?;
    let invoices = list_invoices(db, org_id).await?;
    Ok(AccountBundle {
        user: user.clone(),
        organization,
        subscription,
        entitlements,
        billing_contact,
        invoices,
    })
}

/// Idempotent repair for orgs created outside `register()` (missing subscription rows, etc.).
pub async fn ensure_org_defaults(db: &AccountDb, org_id: &str) -> Result<()> {
    let (period_start, period_end) = current_billing_period();
    let limits = limits_for_plan(db, "free").await;

    let sub_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM subscriptions WHERE organization_id = ?")
            .bind(org_id)
            .fetch_one(db.pool())
            .await?;
    if sub_count == 0 {
        sqlx::query(
            r#"
            INSERT INTO subscriptions (organization_id, plan, status, billing_cycle, period_start, period_end)
            VALUES (?, 'free', 'active', 'monthly', ?, ?)
            "#,
        )
        .bind(org_id)
        .bind(period_start)
        .bind(period_end)
        .execute(db.pool())
        .await?;
    }

    let ent_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM entitlements WHERE organization_id = ?")
            .bind(org_id)
            .fetch_one(db.pool())
            .await?;
    if ent_count == 0 {
        sqlx::query(
            "INSERT INTO entitlements (organization_id, token_limit, api_key_limit, seat_limit, hosted_models_enabled) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(org_id)
        .bind(limits.token_limit)
        .bind(limits.api_key_limit)
        .bind(limits.seat_limit)
        .bind(limits.hosted_models_enabled)
        .execute(db.pool())
        .await?;
    }

    let billing_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM billing_contacts WHERE organization_id = ?")
            .bind(org_id)
            .fetch_one(db.pool())
            .await?;
    if billing_count == 0 {
        sqlx::query("INSERT INTO billing_contacts (organization_id) VALUES (?)")
            .bind(org_id)
            .execute(db.pool())
            .await?;
    }

    Ok(())
}

pub async fn upgrade_plan(db: &AccountDb, org_id: &str, plan: &str) -> Result<SubscriptionView> {
    let plan = match plan {
        "free" | "pro" | "team" | "cloud_5h" => plan,
        _ => return Err(anyhow!("invalid plan tier")),
    };
    if plan == "team" {
        return Err(anyhow!("team plan requires sales contact"));
    }
    let limits = limits_for_plan(db, plan).await;
    let status = subscription_status_for_upgrade(plan);
    let mut tx = db.pool().begin().await?;
    sqlx::query("UPDATE organizations SET plan_tier = ?, updated_at = NOW() WHERE id = ?")
        .bind(plan)
        .bind(org_id)
        .execute(&mut *tx)
        .await?;
    let pass_expires = None::<chrono::DateTime<Utc>>;
    sqlx::query(
        r#"
        UPDATE subscriptions SET plan = ?, status = ?, billing_cycle = ?, pass_expires_at = ?, updated_at = NOW()
        WHERE organization_id = ?
        "#,
    )
    .bind(plan)
    .bind(status)
    .bind("monthly")
    .bind(pass_expires)
    .bind(org_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        UPDATE entitlements SET token_limit = ?, api_key_limit = ?, seat_limit = ?,
          hosted_models_enabled = ?, cloud_unlimited_rate = 0,
          quota_window_secs = ?, calls_limit_per_window = ?, calls_used_in_window = 0,
          quota_window_started_at = CASE WHEN ? > 0 THEN NOW() ELSE NULL END, updated_at = NOW()
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
    tx.commit().await?;
    get_subscription(db, org_id).await
}

pub async fn update_billing_contact(
    db: &AccountDb,
    org_id: &str,
    email: &str,
    company_name: &str,
    tax_id: &str,
) -> Result<BillingContactView> {
    sqlx::query(
        r#"
        INSERT INTO billing_contacts (organization_id, email, company_name, tax_id)
        VALUES (?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE
          email = VALUES(email),
          company_name = VALUES(company_name),
          tax_id = VALUES(tax_id),
          updated_at = CURRENT_TIMESTAMP(3)
        "#,
    )
    .bind(org_id)
    .bind(email)
    .bind(company_name)
    .bind(tax_id)
    .execute(db.pool())
    .await?;
    get_billing_contact(db, org_id, email).await
}

pub async fn update_display_name(db: &AccountDb, user_id: &str, display_name: &str) -> Result<()> {
    let display_name = display_name.trim();
    if display_name.is_empty() {
        return Err(anyhow!("display name required"));
    }
    sqlx::query("UPDATE users SET display_name = ? WHERE id = ?")
        .bind(display_name)
        .bind(user_id)
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn list_org_members(db: &AccountDb, org_id: &str) -> Result<Vec<OrgMemberView>> {
    let rows = sqlx::query(
        r#"
        SELECT id, display_name, email, role, status, last_active_at
        FROM users WHERE organization_id = ? ORDER BY created_at ASC
        "#,
    )
    .bind(org_id)
    .fetch_all(db.pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let last: chrono::DateTime<Utc> = r.get("last_active_at");
            OrgMemberView {
                id: r.get("id"),
                name: r.get("display_name"),
                email: r.get("email"),
                role: r.get("role"),
                status: r.get("status"),
                last_active: last.format("%Y-%m-%d").to_string(),
            }
        })
        .collect())
}

pub async fn list_api_keys(db: &AccountDb, org_id: &str) -> Result<Vec<CloudApiKeyView>> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, prefix, scopes, created_at, expires_at, last_used_at, revoked_at
        FROM cloud_api_keys WHERE organization_id = ? ORDER BY created_at DESC
        "#,
    )
    .bind(org_id)
    .fetch_all(db.pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let scopes_raw: String = r.get("scopes");
            let scopes: Vec<String> = serde_json::from_str(&scopes_raw).unwrap_or_default();
            let created: chrono::DateTime<Utc> = r.get("created_at");
            let expires: Option<chrono::DateTime<Utc>> = r.get("expires_at");
            let last_used: Option<chrono::DateTime<Utc>> = r.get("last_used_at");
            let revoked_at: Option<chrono::DateTime<Utc>> = r.get("revoked_at");
            CloudApiKeyView {
                id: r.get("id"),
                name: r.get("name"),
                prefix: r.get("prefix"),
                scopes,
                created_at: created.to_rfc3339(),
                expires_at: expires.map(|t| t.to_rfc3339()),
                last_used_at: last_used.map(|t| t.to_rfc3339()),
                revoked: revoked_at.is_some(),
            }
        })
        .collect())
}

pub async fn create_api_key(
    db: &AccountDb,
    org_id: &str,
    name: &str,
    expires_days: Option<i64>,
) -> Result<(CloudApiKeyView, String)> {
    let active: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cloud_api_keys WHERE organization_id = ? AND revoked_at IS NULL",
    )
    .bind(org_id)
    .fetch_one(db.pool())
    .await?;
    let limit: i32 =
        sqlx::query_scalar("SELECT api_key_limit FROM entitlements WHERE organization_id = ?")
            .bind(org_id)
            .fetch_one(db.pool())
            .await?;
    if active >= i64::from(limit) {
        return Err(anyhow!("api key limit reached for current plan"));
    }

    let plaintext = new_api_key_plaintext();
    let prefix = plaintext.chars().take(12).collect::<String>();
    let id = format!("ckey_{}", Uuid::new_v4());
    let expires_at = expires_days.map(|d| Utc::now() + Duration::days(d));

    sqlx::query(
        r#"
        INSERT INTO cloud_api_keys (id, organization_id, name, prefix, token_hash, scopes, expires_at)
        VALUES (?, ?, ?, ?, ?, '[]', ?)
        "#,
    )
    .bind(&id)
    .bind(org_id)
    .bind(name)
    .bind(&prefix)
    .bind(hash_token(&plaintext))
    .bind(expires_at)
    .execute(db.pool())
    .await?;

    let keys = list_api_keys(db, org_id).await?;
    let view = keys
        .into_iter()
        .find(|k| k.id == id)
        .ok_or_else(|| anyhow!("key missing"))?;
    Ok((view, plaintext))
}

pub async fn revoke_api_key(db: &AccountDb, org_id: &str, key_id: &str) -> Result<()> {
    sqlx::query(
        "UPDATE cloud_api_keys SET revoked_at = NOW() WHERE id = ? AND organization_id = ?",
    )
    .bind(key_id)
    .bind(org_id)
    .execute(db.pool())
    .await?;
    Ok(())
}

async fn create_session(db: &AccountDb, user_id: &str) -> Result<String> {
    let token = new_session_token();
    let session_id = format!("sess_{}", Uuid::new_v4());
    let expires = Utc::now() + Duration::hours(12);
    sqlx::query("INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)")
        .bind(&session_id)
        .bind(user_id)
        .bind(hash_token(&token))
        .bind(expires)
        .execute(db.pool())
        .await?;
    Ok(token)
}

async fn get_user_by_id(db: &AccountDb, user_id: &str) -> Result<Option<AuthUser>> {
    let row = sqlx::query(
        "SELECT id, organization_id, email, display_name, role FROM users WHERE id = ?",
    )
    .bind(user_id)
    .fetch_optional(db.pool())
    .await?;
    Ok(row.map(|r| AuthUser {
        id: r.get("id"),
        email: r.get("email"),
        display_name: r.get("display_name"),
        role: r.get("role"),
        organization_id: r.get("organization_id"),
    }))
}

async fn get_organization(db: &AccountDb, org_id: &str) -> Result<OrganizationSummary> {
    let row = sqlx::query("SELECT id, name, plan_tier, sso_status FROM organizations WHERE id = ?")
        .bind(org_id)
        .fetch_one(db.pool())
        .await?;
    Ok(OrganizationSummary {
        id: row.get("id"),
        name: row.get("name"),
        plan_tier: row.get("plan_tier"),
        sso_status: row.get("sso_status"),
    })
}

pub async fn get_subscription(db: &AccountDb, org_id: &str) -> Result<SubscriptionView> {
    ensure_org_defaults(db, org_id).await?;
    let _ = crate::billing::refresh_subscription_status(db, org_id).await;
    let row = sqlx::query(
        r#"
        SELECT plan, status, billing_cycle, period_start, period_end, payment_method_bound,
          payment_provider
        FROM subscriptions WHERE organization_id = ?
        "#,
    )
    .bind(org_id)
    .fetch_one(db.pool())
    .await?;
    let period_start: NaiveDate = row.get("period_start");
    let period_end: NaiveDate = row.get("period_end");
    let today = Utc::now().date_naive();
    let days_remaining = (period_end - today).num_days().max(0);
    Ok(SubscriptionView {
        plan: row.get("plan"),
        status: row.get("status"),
        billing_cycle: row.get("billing_cycle"),
        period_start: period_start.to_string(),
        period_end: period_end.to_string(),
        days_remaining,
        payment_method_bound: row.get("payment_method_bound"),
        payment_provider: row.get("payment_provider"),
    })
}

pub async fn get_entitlements(db: &AccountDb, org_id: &str) -> Result<EntitlementsView> {
    ensure_org_defaults(db, org_id).await?;
    let _ = crate::quota::ensure_window_current(db, org_id).await;
    let row = sqlx::query(
        r#"
        SELECT token_limit, api_key_limit, seat_limit, tokens_used, hosted_models_enabled,
          quota_window_secs, calls_limit_per_window, calls_used_in_window, quota_window_started_at
        FROM entitlements WHERE organization_id = ?
        "#,
    )
    .bind(org_id)
    .fetch_one(db.pool())
    .await?;
    let seat_used: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE organization_id = ? AND status = 'active'",
    )
    .bind(org_id)
    .fetch_one(db.pool())
    .await?;
    let calls_limit: i32 = row.get("calls_limit_per_window");
    let calls_used: i32 = row.get("calls_used_in_window");
    let window_secs: i32 = row.get("quota_window_secs");
    let started: Option<chrono::DateTime<Utc>> = row.get("quota_window_started_at");
    let (calls_remaining, quota_resets_at) = if calls_limit > 0 {
        let resets = started
            .map(|t| (t + chrono::Duration::seconds(i64::from(window_secs.max(1)))).to_rfc3339());
        (Some((calls_limit - calls_used).max(0)), resets)
    } else {
        (None, None)
    };
    Ok(EntitlementsView {
        token_limit: row.get("token_limit"),
        api_key_limit: row.get("api_key_limit"),
        seat_limit: row.get("seat_limit"),
        seat_used: seat_used as i32,
        tokens_used: row.get("tokens_used"),
        hosted_models_enabled: row.get("hosted_models_enabled"),
        calls_limit_per_window: if calls_limit > 0 {
            Some(calls_limit)
        } else {
            None
        },
        calls_used_in_window: if calls_limit > 0 {
            Some(calls_used)
        } else {
            None
        },
        calls_remaining,
        quota_window_hours: if window_secs > 0 {
            Some(window_secs / 3600)
        } else {
            None
        },
        quota_resets_at,
    })
}

async fn get_billing_contact(
    db: &AccountDb,
    org_id: &str,
    fallback_email: &str,
) -> Result<BillingContactView> {
    let row = sqlx::query(
        "SELECT email, company_name, tax_id FROM billing_contacts WHERE organization_id = ?",
    )
    .bind(org_id)
    .fetch_optional(db.pool())
    .await?;
    Ok(match row {
        Some(r) => BillingContactView {
            email: r
                .get::<Option<String>, _>("email")
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| fallback_email.to_string()),
            company_name: r
                .get::<Option<String>, _>("company_name")
                .unwrap_or_default(),
            tax_id: r.get::<Option<String>, _>("tax_id").unwrap_or_default(),
        },
        None => BillingContactView {
            email: fallback_email.to_string(),
            company_name: String::new(),
            tax_id: String::new(),
        },
    })
}

async fn list_invoices(db: &AccountDb, org_id: &str) -> Result<Vec<InvoiceView>> {
    let rows = sqlx::query(
        r#"
        SELECT id, number, period_start, period_end,
          CAST(COALESCE(amount_fen, ROUND(COALESCE(amount_cny, 0) * 100)) AS SIGNED) AS amount_fen,
          COALESCE(NULLIF(currency, ''), 'CNY') AS currency, status
        FROM invoices WHERE organization_id = ? ORDER BY created_at DESC
        "#,
    )
    .bind(org_id)
    .fetch_all(db.pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| InvoiceView {
            id: r.get("id"),
            number: r.get("number"),
            period_start: r.get::<NaiveDate, _>("period_start").to_string(),
            period_end: r.get::<NaiveDate, _>("period_end").to_string(),
            amount_fen: r.get::<i32, _>("amount_fen"),
            currency: r.get("currency"),
            status: r.get("status"),
        })
        .collect())
}

fn current_billing_period() -> (NaiveDate, NaiveDate) {
    let today = Utc::now().date_naive();
    let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
    let end = if today.month() == 12 {
        NaiveDate::from_ymd_opt(today.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1)
    }
    .unwrap_or(today)
    .pred_opt()
    .unwrap_or(today);
    (start, end)
}
