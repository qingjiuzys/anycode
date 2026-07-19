use crate::auth::{
    hash_password, hash_token, new_session_token, password_needs_rehash, verify_password,
};
use crate::db::AccountDb;
use anyhow::Result;
use chrono::{Duration, Utc};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct AdminUserView {
    pub id: String,
    pub email: String,
    pub role: String,
}

pub async fn bootstrap_admin_if_needed(
    db: &AccountDb,
    email: Option<&str>,
    password: Option<&str>,
) -> Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admin_users")
        .fetch_one(db.pool())
        .await?;
    if count > 0 {
        return Ok(());
    }
    let Some(email) = email.filter(|s| !s.trim().is_empty()) else {
        return Ok(());
    };
    let Some(password) = password.filter(|s| !s.trim().is_empty()) else {
        return Ok(());
    };
    create_admin_user(db, email, password, "superadmin").await?;
    tracing::info!("bootstrapped admin user {}", email);
    Ok(())
}

pub async fn create_admin_user(
    db: &AccountDb,
    email: &str,
    password: &str,
    role: &str,
) -> Result<AdminUserView> {
    let id = format!("adm_{}", Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO admin_users (id, email, password_hash, role, status)
        VALUES (?, ?, ?, ?, 'active')
        "#,
    )
    .bind(&id)
    .bind(email)
    .bind(hash_password(password))
    .bind(role)
    .execute(db.pool())
    .await?;
    Ok(AdminUserView {
        id,
        email: email.to_string(),
        role: role.to_string(),
    })
}

pub async fn admin_login(db: &AccountDb, email: &str, password: &str) -> Result<Option<String>> {
    let row = sqlx::query("SELECT id, password_hash, status FROM admin_users WHERE email = ?")
        .bind(email)
        .fetch_optional(db.pool())
        .await?;
    let Some(r) = row else {
        return Ok(None);
    };
    let status: String = r.get("status");
    if status != "active" {
        return Ok(None);
    }
    let hash: String = r.get("password_hash");
    if !verify_password(password, &hash) {
        return Ok(None);
    }
    let admin_id: String = r.get("id");
    if password_needs_rehash(&hash) {
        sqlx::query("UPDATE admin_users SET password_hash = ? WHERE id = ?")
            .bind(hash_password(password))
            .bind(&admin_id)
            .execute(db.pool())
            .await?;
    }
    let token = new_session_token();
    let session_id = format!("adm_sess_{}", Uuid::new_v4());
    let expires = Utc::now() + Duration::days(7);
    sqlx::query(
        "INSERT INTO admin_sessions (id, admin_user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&session_id)
    .bind(&admin_id)
    .bind(hash_token(&token))
    .bind(expires)
    .execute(db.pool())
    .await?;
    Ok(Some(token))
}

pub async fn resolve_admin_session(db: &AccountDb, token: &str) -> Result<Option<AdminUserView>> {
    let row = sqlx::query(
        r#"
        SELECT u.id, u.email, u.role
        FROM admin_sessions s
        JOIN admin_users u ON u.id = s.admin_user_id
        WHERE s.token_hash = ? AND s.expires_at > NOW() AND u.status = 'active'
        "#,
    )
    .bind(hash_token(token))
    .fetch_optional(db.pool())
    .await?;
    Ok(row.map(|r| AdminUserView {
        id: r.get("id"),
        email: r.get("email"),
        role: r.get("role"),
    }))
}

pub async fn write_audit_log(
    db: &AccountDb,
    admin_user_id: &str,
    action: &str,
    resource_type: &str,
    resource_id: Option<&str>,
    details: Option<serde_json::Value>,
) -> Result<()> {
    let id = format!("aud_{}", Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO admin_audit_logs (id, admin_user_id, action, resource_type, resource_id, details)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(admin_user_id)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(details)
    .execute(db.pool())
    .await?;
    Ok(())
}
