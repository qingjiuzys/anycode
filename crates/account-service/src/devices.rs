use crate::auth::{hash_token, new_session_token};
use crate::db::AccountDb;
use crate::models::{AuthUser, LinkedDeviceView};
use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use sqlx::Row;
use uuid::Uuid;

const DEVICE_CODE_TTL_SECS: i64 = 600;

pub struct DeviceLinkStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: i64,
    pub interval: i64,
}

pub struct DeviceLinkTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub user: AuthUser,
    pub entitlements: crate::models::EntitlementsView,
}

fn random_code(len: usize) -> String {
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        let n = (Uuid::new_v4().as_u128() % 36) as u8;
        let c = if n < 10 { b'0' + n } else { b'A' + (n - 10) };
        out.push(c as char);
    }
    out
}

pub async fn start_device_link(
    db: &AccountDb,
    device_name: Option<&str>,
    portal_url: &str,
) -> Result<DeviceLinkStart> {
    let device_code = format!("dev_{}", Uuid::new_v4());
    let user_code = format!("{}-{}", random_code(4), random_code(4));
    let link_id = format!("dlink_{}", Uuid::new_v4());
    let expires = Utc::now() + Duration::seconds(DEVICE_CODE_TTL_SECS);

    sqlx::query(
        r#"
        INSERT INTO device_links (id, user_id, device_code_hash, user_code, device_name, status, expires_at)
        VALUES (?, NULL, ?, ?, ?, 'pending', ?)
        "#,
    )
    .bind(&link_id)
    .bind(hash_token(&device_code))
    .bind(&user_code)
    .bind(device_name.unwrap_or("anyCode"))
    .bind(expires)
    .execute(db.pool())
    .await?;

    let verification_uri = format!(
        "{}/login?device_code={}&redirect_uri=anycode%3A%2F%2Flink",
        portal_url.trim_end_matches('/'),
        device_code,
    );

    Ok(DeviceLinkStart {
        device_code,
        user_code,
        verification_uri,
        expires_in: DEVICE_CODE_TTL_SECS,
        interval: 2,
    })
}

pub async fn poll_device_link(
    db: &AccountDb,
    device_code: &str,
) -> Result<Option<DeviceLinkTokens>> {
    let mut tx = db.pool().begin().await?;
    let row = sqlx::query(
        r#"
        SELECT dl.id, dl.user_id, dl.status, dl.expires_at, dl.device_name,
               u.email, u.display_name, u.role, u.organization_id
        FROM device_links dl
        JOIN users u ON u.id = dl.user_id
        WHERE dl.device_code_hash = ? FOR UPDATE
        "#,
    )
    .bind(hash_token(device_code))
    .fetch_optional(&mut *tx)
    .await?;

    let Some(r) = row else {
        return Ok(None);
    };

    let status: String = r.get("status");
    let expires: chrono::DateTime<Utc> = r.get("expires_at");
    if expires < Utc::now() {
        return Err(anyhow!("device code expired"));
    }
    if status != "approved" {
        return Ok(None);
    }

    let user = AuthUser {
        id: r.get("user_id"),
        email: r.get("email"),
        display_name: r.get("display_name"),
        role: r.get("role"),
        organization_id: r.get("organization_id"),
    };

    let access_token = new_session_token();
    let refresh_token = format!("refr_{}", Uuid::new_v4());
    let device_id = format!("ldev_{}", Uuid::new_v4());
    let device_name: String = r
        .get::<Option<String>, _>("device_name")
        .unwrap_or_else(|| "anyCode".into());

    let refresh_expires = Utc::now() + Duration::days(7);
    let token_family_id = format!("tfam_{}", Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO linked_devices
          (id, user_id, device_name, refresh_token_hash, refresh_expires_at, token_family_id)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&device_id)
    .bind(&user.id)
    .bind(&device_name)
    .bind(hash_token(&refresh_token))
    .bind(refresh_expires)
    .bind(token_family_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM device_links WHERE device_code_hash = ?")
        .bind(hash_token(device_code))
        .execute(&mut *tx)
        .await?;

    let session_id = format!("sess_{}", Uuid::new_v4());
    let session_expires = Utc::now() + Duration::minutes(15);
    sqlx::query(
        "INSERT INTO sessions (id, user_id, token_hash, expires_at, session_kind) VALUES (?, ?, ?, ?, 'device')",
    )
    .bind(&session_id)
    .bind(&user.id)
    .bind(hash_token(&access_token))
    .bind(session_expires)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let entitlements = crate::store::get_entitlements(db, &user.organization_id).await?;

    Ok(Some(DeviceLinkTokens {
        access_token,
        refresh_token,
        user,
        entitlements,
    }))
}

pub async fn approve_device_link(db: &AccountDb, user_id: &str, device_code: &str) -> Result<()> {
    let updated = sqlx::query(
        r#"
        UPDATE device_links SET user_id = ?, status = 'approved', approved_at = NOW()
        WHERE device_code_hash = ? AND user_id IS NULL
          AND status = 'pending' AND expires_at > NOW()
        "#,
    )
    .bind(user_id)
    .bind(hash_token(device_code))
    .execute(db.pool())
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(anyhow!("invalid or expired device code"));
    }
    Ok(())
}

pub async fn list_linked_devices(db: &AccountDb, user_id: &str) -> Result<Vec<LinkedDeviceView>> {
    let rows = sqlx::query(
        r#"
        SELECT id, device_name, platform, last_seen_at, revoked_at, created_at
        FROM linked_devices WHERE user_id = ? ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(db.pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let last: chrono::DateTime<Utc> = r.get("last_seen_at");
            let created: chrono::DateTime<Utc> = r.get("created_at");
            let revoked: Option<chrono::DateTime<Utc>> = r.get("revoked_at");
            LinkedDeviceView {
                id: r.get("id"),
                device_name: r.get("device_name"),
                platform: r.get("platform"),
                last_seen_at: last.to_rfc3339(),
                created_at: created.to_rfc3339(),
                revoked: revoked.is_some(),
            }
        })
        .collect())
}

pub async fn revoke_linked_device(db: &AccountDb, user_id: &str, device_id: &str) -> Result<()> {
    sqlx::query(
        "UPDATE linked_devices SET revoked_at = NOW(), refresh_token_hash = CONCAT('revoked:', id, ':', UNIX_TIMESTAMP()) WHERE id = ? AND user_id = ?",
    )
        .bind(device_id)
        .bind(user_id)
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn refresh_device_session(
    db: &AccountDb,
    refresh_token: &str,
) -> Result<Option<DeviceLinkTokens>> {
    let presented_hash = hash_token(refresh_token);
    let mut tx = db.pool().begin().await?;
    let replayed_family: Option<String> = sqlx::query_scalar(
        "SELECT token_family_id FROM linked_devices WHERE previous_refresh_token_hash = ? AND revoked_at IS NULL LIMIT 1 FOR UPDATE",
    )
    .bind(&presented_hash)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(family) = replayed_family {
        sqlx::query("UPDATE linked_devices SET revoked_at = NOW() WHERE token_family_id = ?")
            .bind(family)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Err(anyhow!(
            "refresh token reuse detected; token family revoked"
        ));
    }

    let row = sqlx::query(
        r#"
        SELECT ld.id, ld.user_id, ld.token_family_id,
               u.email, u.display_name, u.role, u.organization_id
        FROM linked_devices ld
        JOIN users u ON u.id = ld.user_id
        WHERE ld.refresh_token_hash = ? AND ld.revoked_at IS NULL
          AND ld.refresh_expires_at > NOW() AND u.status = 'active'
        FOR UPDATE
        "#,
    )
    .bind(&presented_hash)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(r) = row else {
        tx.rollback().await?;
        return Ok(None);
    };

    let user = AuthUser {
        id: r.get("user_id"),
        email: r.get("email"),
        display_name: r.get("display_name"),
        role: r.get("role"),
        organization_id: r.get("organization_id"),
    };

    let access_token = new_session_token();
    let new_refresh_token = format!("refr_{}", Uuid::new_v4());
    let session_id = format!("sess_{}", Uuid::new_v4());
    let session_expires = Utc::now() + Duration::minutes(15);
    sqlx::query(
        "INSERT INTO sessions (id, user_id, token_hash, expires_at, session_kind) VALUES (?, ?, ?, ?, 'device')",
    )
    .bind(&session_id)
    .bind(&user.id)
    .bind(hash_token(&access_token))
    .bind(session_expires)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE linked_devices SET previous_refresh_token_hash = refresh_token_hash,
          previous_refresh_used_at = NOW(), refresh_token_hash = ?,
          refresh_expires_at = DATE_ADD(NOW(), INTERVAL 7 DAY),
          refresh_generation = refresh_generation + 1, last_seen_at = NOW()
        WHERE id = ?
        "#,
    )
    .bind(hash_token(&new_refresh_token))
    .bind(r.get::<String, _>("id"))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let entitlements = crate::store::get_entitlements(db, &user.organization_id).await?;

    Ok(Some(DeviceLinkTokens {
        access_token,
        refresh_token: new_refresh_token,
        user,
        entitlements,
    }))
}
