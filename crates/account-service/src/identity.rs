use crate::{auth::hash_token, crypto, db::AccountDb};
use anyhow::{anyhow, Result};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct IdentityStatusView {
    pub status: String,
    pub legal_name_masked: Option<String>,
    pub id_number_masked: Option<String>,
    pub submitted_at: Option<String>,
    pub reviewed_at: Option<String>,
    pub rejection_reason: Option<String>,
    pub document_upload_supported: bool,
}

const ID_WEIGHTS: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
const ID_CHECK_DIGITS: [char; 11] = ['1', '0', 'X', '9', '8', '7', '6', '5', '4', '3', '2'];

fn normalize_id_number(value: &str) -> String {
    value.trim().to_uppercase()
}

fn validate_id_birth_date(id: &str) -> bool {
    if id.len() != 18 {
        return false;
    }
    let y: u32 = id[6..10].parse().unwrap_or(0);
    let m: u32 = id[10..12].parse().unwrap_or(0);
    let d: u32 = id[12..14].parse().unwrap_or(0);
    if !(1900..=2100).contains(&y) || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return false;
    }
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => d <= 31,
        4 | 6 | 9 | 11 => d <= 30,
        2 => {
            let leap = (!y.is_multiple_of(100) && y.is_multiple_of(4)) || y.is_multiple_of(400);
            d <= if leap { 29 } else { 28 }
        }
        _ => false,
    }
}

fn validate_id_checksum(id: &str) -> bool {
    if id.len() != 18 {
        return false;
    }
    let mut sum = 0u32;
    for (i, ch) in id.chars().take(17).enumerate() {
        let digit = ch.to_digit(10).unwrap_or(99);
        sum += digit * ID_WEIGHTS[i];
    }
    let expected = ID_CHECK_DIGITS[(sum % 11) as usize];
    id.chars().nth(17) == Some(expected)
}

pub fn validate_id_number(value: &str) -> bool {
    let value = normalize_id_number(value);
    value.len() == 18
        && value
            .chars()
            .enumerate()
            .all(|(i, c)| c.is_ascii_digit() || (i == 17 && c == 'X'))
        && validate_id_birth_date(&value)
        && validate_id_checksum(&value)
}

fn mask_name(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => format!("{first}{}", "*".repeat(chars.count().max(1))),
        None => String::new(),
    }
}

pub async fn submit(
    db: &AccountDb,
    user_id: &str,
    legal_name: &str,
    id_number: &str,
    secret: &str,
) -> Result<()> {
    let legal_name = legal_name.trim();
    let id_number = normalize_id_number(id_number);
    if legal_name.chars().count() < 2 || !validate_id_number(&id_number) {
        return Err(anyhow!(
            "valid legal name and 18-digit identity number required"
        ));
    }
    let (name_ct, name_nonce) = crypto::encrypt_secret(legal_name, secret)?;
    let (id_ct, id_nonce) = crypto::encrypt_secret(&id_number, secret)?;
    let fingerprint = hash_token(&format!("{secret}:{id_number}"));
    let last4 = id_number
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();

    let duplicate: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM identity_reviews WHERE id_number_fingerprint = ? AND user_id <> ? AND status = 'approved'",
    )
    .bind(&fingerprint)
    .bind(user_id)
    .fetch_one(db.pool())
    .await?;
    if duplicate > 0 {
        return Err(anyhow!(
            "this identity number is already linked to another account"
        ));
    }

    let result = sqlx::query(
        r#"
        INSERT INTO identity_reviews
          (id, user_id, status, legal_name_ciphertext, legal_name_nonce,
           id_number_ciphertext, id_number_nonce, id_number_fingerprint, id_number_last4,
           submitted_at, reviewed_at)
        VALUES (?, ?, 'approved', ?, ?, ?, ?, ?, ?, NOW(), NOW())
        ON DUPLICATE KEY UPDATE
          status = 'approved', legal_name_ciphertext = VALUES(legal_name_ciphertext),
          legal_name_nonce = VALUES(legal_name_nonce),
          id_number_ciphertext = VALUES(id_number_ciphertext),
          id_number_nonce = VALUES(id_number_nonce),
          id_number_fingerprint = VALUES(id_number_fingerprint),
          id_number_last4 = VALUES(id_number_last4), submitted_at = NOW(),
          reviewed_at = NOW(), reviewer_admin_id = NULL, rejection_reason = NULL
        "#,
    )
    .bind(format!("kyc_{}", Uuid::new_v4()))
    .bind(user_id)
    .bind(name_ct)
    .bind(name_nonce)
    .bind(id_ct)
    .bind(id_nonce)
    .bind(fingerprint)
    .bind(last4)
    .execute(db.pool())
    .await;

    if let Err(e) = result {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.code().as_deref() == Some("23000") {
                return Err(anyhow!(
                    "this identity number is already linked to another account"
                ));
            }
        }
        return Err(e.into());
    }

    sqlx::query("UPDATE users SET identity_status = 'approved', status = 'active' WHERE id = ?")
        .bind(user_id)
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn status(db: &AccountDb, user_id: &str, secret: &str) -> Result<IdentityStatusView> {
    let user_status: String = sqlx::query_scalar("SELECT identity_status FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_one(db.pool())
        .await?;
    let row = sqlx::query(
        "SELECT legal_name_ciphertext, legal_name_nonce, id_number_last4, submitted_at, reviewed_at, rejection_reason FROM identity_reviews WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(db.pool())
    .await?;
    let Some(row) = row else {
        return Ok(IdentityStatusView {
            status: user_status,
            legal_name_masked: None,
            id_number_masked: None,
            submitted_at: None,
            reviewed_at: None,
            rejection_reason: None,
            document_upload_supported: false,
        });
    };
    let name = crypto::decrypt_secret(
        row.get("legal_name_ciphertext"),
        row.get("legal_name_nonce"),
        secret,
    )?;
    let submitted: chrono::DateTime<chrono::Utc> = row.get("submitted_at");
    let reviewed: Option<chrono::DateTime<chrono::Utc>> = row.get("reviewed_at");
    Ok(IdentityStatusView {
        status: user_status,
        legal_name_masked: Some(mask_name(&name)),
        id_number_masked: Some(format!(
            "**************{}",
            row.get::<String, _>("id_number_last4")
        )),
        submitted_at: Some(submitted.to_rfc3339()),
        reviewed_at: reviewed.map(|v| v.to_rfc3339()),
        rejection_reason: row.get("rejection_reason"),
        document_upload_supported: false,
    })
}

pub async fn review(
    db: &AccountDb,
    admin_id: &str,
    review_id: &str,
    approved: bool,
    reason: Option<&str>,
) -> Result<()> {
    if !approved && reason.unwrap_or("").trim().is_empty() {
        return Err(anyhow!("rejection reason required"));
    }
    let mut tx = db.pool().begin().await?;
    let status = if approved { "approved" } else { "rejected" };
    let user_id: Option<String> = sqlx::query_scalar(
        "SELECT user_id FROM identity_reviews WHERE id = ? AND status = 'pending' FOR UPDATE",
    )
    .bind(review_id)
    .fetch_optional(&mut *tx)
    .await?;
    let user_id = user_id.ok_or_else(|| anyhow!("pending identity review not found"))?;
    sqlx::query(
        "UPDATE identity_reviews SET status = ?, reviewer_admin_id = ?, rejection_reason = ?, reviewed_at = NOW() WHERE id = ?",
    )
    .bind(status)
    .bind(admin_id)
    .bind(if approved { None } else { reason })
    .bind(review_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE users SET identity_status = ?, status = ? WHERE id = ?")
        .bind(status)
        .bind(if approved {
            "active"
        } else {
            "identity_pending"
        })
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_checksum_and_rejects_invalid() {
        // Well-known test vector format (checksum valid for structure)
        assert!(validate_id_number("11010519491231002X"));
        assert!(!validate_id_number("110105194912310021"));
        assert!(!validate_id_number("123"));
    }
}
