//! Local dashboard user session (UI login, distinct from API Bearer tokens).

use crate::db::DashboardDb;
use crate::schema::{LOCAL_ORG_ID, LOCAL_USER_ID};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub organization_id: String,
    pub auth_method: String,
}

#[derive(Clone, Default)]
pub struct SessionStore {
    inner: Arc<RwLock<HashMap<String, String>>>,
}

impl SessionStore {
    pub fn create(&self, user_id: &str) -> String {
        let token = format!("sess_{}", Uuid::new_v4());
        self.inner
            .write()
            .unwrap()
            .insert(token.clone(), user_id.to_string());
        token
    }

    pub fn resolve(&self, token: &str) -> Option<String> {
        self.inner.read().unwrap().get(token).cloned()
    }

    pub fn revoke(&self, token: &str) {
        self.inner.write().unwrap().remove(token);
    }
}

pub async fn get_user_by_id(db: &DashboardDb, user_id: &str) -> Result<Option<AuthUser>> {
    let row = sqlx::query(
        r#"
        SELECT id, organization_id, email, display_name, role
        FROM users WHERE id = ?
        "#,
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
        auth_method: "session".into(),
    }))
}

pub async fn local_trusted_user(db: &DashboardDb) -> Result<AuthUser> {
    get_user_by_id(db, LOCAL_USER_ID)
        .await?
        .ok_or_else(|| anyhow::anyhow!("local user missing"))
        .map(|mut u| {
            u.auth_method = "local_trusted".into();
            u
        })
}

pub async fn login(db: &DashboardDb, email: &str, password: &str) -> Result<Option<AuthUser>> {
    let row = sqlx::query(
        r#"
        SELECT id, organization_id, email, display_name, role, password_hash
        FROM users
        WHERE organization_id = ? AND email = ?
        "#,
    )
    .bind(LOCAL_ORG_ID)
    .bind(email.trim())
    .fetch_optional(db.pool())
    .await?;
    let Some(r) = row else {
        return Ok(None);
    };
    let hash: Option<String> = r.get("password_hash");
    if let Some(h) = hash.filter(|s| !s.is_empty()) {
        if !verify_password(password, &h) {
            return Ok(None);
        }
    }
    Ok(Some(AuthUser {
        id: r.get("id"),
        email: r.get("email"),
        display_name: r.get("display_name"),
        role: r.get("role"),
        organization_id: r.get("organization_id"),
        auth_method: "session".into(),
    }))
}

fn verify_password(password: &str, hash: &str) -> bool {
    // Placeholder: production would use argon2; local dev may leave hash empty.
    hash == password || hash.is_empty()
}

pub const SESSION_COOKIE: &str = "dw_session";
