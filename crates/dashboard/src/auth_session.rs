//! Local dashboard user session (UI login, distinct from API Bearer tokens).

use crate::db::DashboardDb;
use crate::schema::{LOCAL_ORG_ID, LOCAL_USER_ID};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
    let hash = hash.trim();
    if hash.is_empty() {
        return true;
    }

    if let Some((salt, expected)) = parse_sha256_password_hash(hash) {
        let mut hasher = Sha256::new();
        hasher.update(salt.as_bytes());
        hasher.update(b":");
        hasher.update(password.as_bytes());
        let actual = hex_lower(&hasher.finalize());
        return constant_time_eq(actual.as_bytes(), expected.as_bytes());
    }

    // Backward compatibility for existing local databases created before
    // password hashes were supported. New configured passwords should use
    // `sha256$<salt>$<hex_sha256(salt:password)>`.
    constant_time_eq(password.as_bytes(), hash.as_bytes())
}

pub const SESSION_COOKIE: &str = "dw_session";

fn parse_sha256_password_hash(hash: &str) -> Option<(&str, &str)> {
    let mut parts = hash.split('$');
    let scheme = parts.next()?;
    let salt = parts.next()?;
    let expected = parts.next()?;
    if parts.next().is_some()
        || scheme != "sha256"
        || salt.is_empty()
        || expected.len() != 64
        || !expected.as_bytes().iter().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    Some((salt, expected))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let max_len = a.len().max(b.len());
    let mut diff = a.len() ^ b.len();
    for i in 0..max_len {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        diff |= (av ^ bv) as usize;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::{hex_lower, verify_password};
    use sha2::{Digest, Sha256};

    fn sha256_password_hash(salt: &str, password: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(salt.as_bytes());
        hasher.update(b":");
        hasher.update(password.as_bytes());
        format!("sha256${salt}${}", hex_lower(&hasher.finalize()))
    }

    #[test]
    fn verifies_sha256_password_hashes() {
        let hash = sha256_password_hash("tenant-a", "correct horse");
        assert!(verify_password("correct horse", &hash));
        assert!(!verify_password("wrong horse", &hash));
    }

    #[test]
    fn keeps_legacy_plaintext_compatibility() {
        assert!(verify_password("old-password", "old-password"));
        assert!(!verify_password("old-password", "other-password"));
    }

    #[test]
    fn rejects_malformed_sha256_hashes() {
        assert!(!verify_password("pw", "sha256$salt$not-hex"));
        assert!(!verify_password("pw", "sha256$$0123456789abcdef"));
    }
}
